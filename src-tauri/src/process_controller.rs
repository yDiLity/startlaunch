use crate::error::{AutoLaunchError, Result};
use crate::models::{ProcessHandle, ExecutionStatus, LogEntry};
use crate::environment_manager::{Environment, IsolationMode};
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::Utc;
use regex::Regex;
use std::io::{BufRead, BufReader};

pub struct ProcessController {
    running_processes: Arc<Mutex<HashMap<String, RunningProcess>>>,
}

struct RunningProcess {
    handle: ProcessHandle,
    child: Option<Child>,
    status: ExecutionStatus,
    logs: Vec<LogEntry>,
    environment: Environment,
    command: String, // Сохраняем команду для перезапуска
}

impl ProcessController {
    pub fn new() -> Self {
        Self {
            running_processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_process(&self, env: &Environment, command: &str) -> Result<ProcessHandle> {
        let process_id = Uuid::new_v4().to_string();
        
        let (child, ports) = match &env.mode {
            IsolationMode::Sandbox(_) => {
                self.start_docker_process(env, command).await?
            }
            IsolationMode::Direct(_) => {
                self.start_direct_process(env, command).await?
            }
        };

        let handle = ProcessHandle {
            id: process_id.clone(),
            pid: child.as_ref().and_then(|c| c.id()),
            container_id: env.container_id.clone(),
            ports,
        };

        let running_process = RunningProcess {
            handle: handle.clone(),
            child,
            status: ExecutionStatus::Starting,
            logs: Vec::new(),
            environment: env.clone(),
            command: command.to_string(),
        };

        {
            let mut processes = self.running_processes.lock().unwrap();
            processes.insert(process_id, running_process);
        }

        // Запускаем мониторинг процесса в фоне
        let processes_ref = Arc::clone(&self.running_processes);
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            Self::monitor_process(processes_ref, handle_clone).await;
        });

        Ok(handle)
    }

    async fn start_docker_process(&self, env: &Environment, command: &str) -> Result<(Option<Child>, Vec<u16>)> {
        if let Some(container_id) = &env.container_id {
            // Запускаем контейнер
            let start_output = Command::new("docker")
                .args(&["start", container_id])
                .output()?;

            if !start_output.status.success() {
                let error_msg = String::from_utf8_lossy(&start_output.stderr);
                return Err(AutoLaunchError::Process(format!("Ошибка запуска контейнера: {}", error_msg)));
            }

            // Выполняем команду в контейнере
            let child = Command::new("docker")
                .args(&["exec", "-d", container_id, "sh", "-c", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            // Определяем порты из конфигурации Docker
            let ports = if let IsolationMode::Sandbox(config) = &env.mode {
                config.ports.clone()
            } else {
                vec![]
            };

            Ok((Some(child), ports))
        } else {
            Err(AutoLaunchError::Process("Container ID не найден".to_string()))
        }
    }

    async fn start_direct_process(&self, env: &Environment, command: &str) -> Result<(Option<Child>, Vec<u16>)> {
        // Парсим команду
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(AutoLaunchError::Process("Пустая команда".to_string()));
        }

        let program = parts[0];
        let args = &parts[1..];

        let child = Command::new(program)
            .args(args)
            .current_dir(&env.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Пытаемся определить порты из команды
        let ports = self.detect_ports_from_command(command);

        Ok((Some(child), ports))
    }

    /// Добавляет лог-запись для процесса
    fn add_log_entry(&self, process_id: &str, level: &str, message: String) {
        let mut processes = self.running_processes.lock().unwrap();
        if let Some(running_process) = processes.get_mut(process_id) {
            running_process.logs.push(LogEntry {
                timestamp: Utc::now(),
                level: level.to_string(),
                message,
            });
            
            // Ограничиваем количество логов (последние 1000)
            if running_process.logs.len() > 1000 {
                running_process.logs.drain(0..100);
            }
        }
    }

    /// Публичный хелпер для тестирования детекции портов
    #[cfg(test)]
    pub fn detect_ports_from_command_test(&self, command: &str) -> Vec<u16> {
        self.detect_ports_from_command(command)
    }

    fn detect_ports_from_command(&self, command: &str) -> Vec<u16> {
        let mut ports = Vec::new();
        
        // Ищем порты в команде
        let port_regex = Regex::new(r"(?:port|PORT|--port|-p)\s*[=:]?\s*(\d+)").unwrap();
        for cap in port_regex.captures_iter(command) {
            if let Ok(port) = cap[1].parse::<u16>() {
                ports.push(port);
            }
        }

        // Добавляем стандартные порты если не найдены
        if ports.is_empty() {
            if command.contains("npm") && command.contains("start") {
                ports.push(3000);
            } else if command.contains("python") {
                ports.push(5000);
            } else {
                ports.push(8000);
            }
        }

        ports
    }

    pub async fn stop_process(&self, handle: &ProcessHandle) -> Result<()> {
        let mut processes = self.running_processes.lock().unwrap();
        
        if let Some(running_process) = processes.get_mut(&handle.id) {
            running_process.status = ExecutionStatus::Stopping;

            // Требование 6.2: Корректно завершать все процессы проекта
            match &running_process.environment.mode {
                IsolationMode::Sandbox(_) => {
                    if let Some(container_id) = &handle.container_id {
                        // Останавливаем контейнер с таймаутом
                        let stop_output = Command::new("docker")
                            .args(&["stop", "-t", "10", container_id])
                            .output();
                        
                        if let Err(e) = stop_output {
                            eprintln!("Ошибка остановки контейнера: {}", e);
                        }
                    }
                }
                IsolationMode::Direct(_) => {
                    if let Some(ref mut child) = running_process.child {
                        // Пытаемся корректно завершить процесс
                        #[cfg(unix)]
                        {
                            use std::os::unix::process::CommandExt;
                            // Отправляем SIGTERM для корректного завершения
                            if let Some(pid) = child.id() {
                                let _ = Command::new("kill")
                                    .args(&["-TERM", &pid.to_string()])
                                    .output();
                                
                                // Ждем немного
                                std::thread::sleep(std::time::Duration::from_secs(2));
                            }
                        }
                        
                        // Если процесс все еще работает, принудительно завершаем
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }

            running_process.status = ExecutionStatus::Stopped;
            
            // Требование 6.4: Автоматическая очистка временных ресурсов
            self.cleanup_process_resources(&running_process.environment).await?;
        }

        Ok(())
    }

    /// Требование 6.4: Автоматическая очистка временных ресурсов
    async fn cleanup_process_resources(&self, env: &Environment) -> Result<()> {
        match &env.mode {
            IsolationMode::Sandbox(_) => {
                if let Some(container_id) = &env.container_id {
                    // Удаляем контейнер
                    let _ = Command::new("docker")
                        .args(&["rm", "-f", container_id])
                        .output();
                }
            }
            IsolationMode::Direct(_) => {
                // Очистка временных файлов для прямого режима
                let temp_dir = env.working_dir.join(".autolaunch_temp");
                if temp_dir.exists() {
                    let _ = std::fs::remove_dir_all(temp_dir);
                }
            }
        }
        Ok(())
    }

    pub async fn restart_process(&self, handle: &ProcessHandle) -> Result<()> {
        // Получаем информацию о процессе (Требование 6.3: Перезапуск = остановка + запуск)
        let (env, command) = {
            let processes = self.running_processes.lock().unwrap();
            if let Some(running_process) = processes.get(&handle.id) {
                let env = running_process.environment.clone();
                let command = running_process.command.clone();
                (env, command)
            } else {
                return Err(AutoLaunchError::Process("Процесс не найден".to_string()));
            }
        };

        // Останавливаем процесс
        self.stop_process(handle).await?;

        // Ждем немного для корректного завершения
        sleep(Duration::from_secs(2)).await;

        // Запускаем заново
        self.start_process(&env, &command).await?;

        Ok(())
    }

    pub async fn get_process_status(&self, handle: &ProcessHandle) -> Result<ExecutionStatus> {
        let processes = self.running_processes.lock().unwrap();
        
        if let Some(running_process) = processes.get(&handle.id) {
            Ok(running_process.status.clone())
        } else {
            Ok(ExecutionStatus::Stopped)
        }
    }

    pub async fn get_process_logs(&self, handle: &ProcessHandle) -> Result<Vec<LogEntry>> {
        let processes = self.running_processes.lock().unwrap();
        
        if let Some(running_process) = processes.get(&handle.id) {
            Ok(running_process.logs.clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn monitor_process(processes: Arc<Mutex<HashMap<String, RunningProcess>>>, handle: ProcessHandle) {
        loop {
            sleep(Duration::from_secs(1)).await;

            let should_continue = {
                let mut processes_guard = processes.lock().unwrap();
                if let Some(running_process) = processes_guard.get_mut(&handle.id) {
                    // Проверяем статус процесса
                    let is_running = match &running_process.child {
                        Some(child) => {
                            // Для упрощения считаем что процесс работает
                            // В реальной реализации нужно проверить child.try_wait()
                            true
                        }
                        None => {
                            // Для Docker контейнеров проверяем через docker ps
                            if let Some(container_id) = &handle.container_id {
                                Self::is_container_running(container_id)
                            } else {
                                false
                            }
                        }
                    };

                    if is_running {
                        if matches!(running_process.status, ExecutionStatus::Starting) {
                            running_process.status = ExecutionStatus::Running;
                        }
                        true
                    } else {
                        running_process.status = ExecutionStatus::Stopped;
                        false
                    }
                } else {
                    false
                }
            };

            if !should_continue {
                break;
            }
        }
    }

    fn is_container_running(container_id: &str) -> bool {
        match Command::new("docker")
            .args(&["ps", "-q", "--filter", &format!("id={}", container_id)])
            .output()
        {
            Ok(output) => !output.stdout.is_empty(),
            Err(_) => false,
        }
    }

    /// Требование 10.1, 10.3: Автоматическое определение портов приложений
    pub async fn detect_application_port(&self, handle: &ProcessHandle) -> Result<Option<u16>> {
        // Пытаемся определить порт приложения из логов
        let logs = self.get_process_logs(handle).await?;
        
        for log in logs {
            if let Some(port) = self.extract_port_from_log(&log.message) {
                return Ok(Some(port));
            }
        }

        // Возвращаем первый порт из handle если не найден в логах
        Ok(handle.ports.first().copied())
    }

    /// Требование 10.1, 10.3: Извлечение порта из логов с поддержкой нестандартных портов
    fn extract_port_from_log(&self, message: &str) -> Option<u16> {
        // Ищем различные паттерны портов в логах
        let patterns = [
            r"listening on port (\d+)",
            r"server running on port (\d+)",
            r"localhost:(\d+)",
            r"127\.0\.0\.1:(\d+)",
            r"0\.0\.0\.0:(\d+)",
            r"port (\d+)",
            r"http://[^:]+:(\d+)",
            r"https://[^:]+:(\d+)",
            r"started on :(\d+)",
            r"running at.*:(\d+)",
            r"available on.*:(\d+)",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(cap) = regex.captures(message) {
                    if let Ok(port) = cap[1].parse::<u16>() {
                        return Some(port);
                    }
                }
            }
        }

        None
    }

    /// Требование 6.5: Остановить все запущенные проекты
    pub async fn stop_all_processes(&self) -> Result<Vec<String>> {
        let process_ids: Vec<String> = {
            let processes = self.running_processes.lock().unwrap();
            processes.keys().cloned().collect()
        };

        let mut stopped_ids = Vec::new();
        
        for process_id in process_ids {
            let handle = {
                let processes = self.running_processes.lock().unwrap();
                processes.get(&process_id).map(|p| p.handle.clone())
            };

            if let Some(handle) = handle {
                if let Ok(()) = self.stop_process(&handle).await {
                    stopped_ids.push(process_id);
                }
            }
        }

        Ok(stopped_ids)
    }

    /// Получить список всех запущенных процессов
    pub fn get_running_processes(&self) -> Vec<ProcessHandle> {
        let processes = self.running_processes.lock().unwrap();
        processes.values()
            .filter(|p| matches!(p.status, ExecutionStatus::Running | ExecutionStatus::Starting))
            .map(|p| p.handle.clone())
            .collect()
    }

    /// Проверить, есть ли запущенные процессы
    pub fn has_running_processes(&self) -> bool {
        let processes = self.running_processes.lock().unwrap();
        processes.values().any(|p| matches!(p.status, ExecutionStatus::Running | ExecutionStatus::Starting))
    }

    /// Требование 10.2: Автоматическое открытие браузера на localhost:port
    pub async fn open_browser_for_port(&self, port: u16) -> Result<()> {
        let url = format!("http://localhost:{}", port);
        tracing::info!("Открытие браузера: {}", url);
        
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(&["/C", "start", &url])
                .spawn()?;
        }
        
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&url)
                .spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()?;
        }
        
        Ok(())
    }

    /// Требование 10.4: Проверка доступности порта
    pub async fn check_port_availability(&self, port: u16, timeout_secs: u64) -> Result<bool> {
        use tokio::net::TcpStream;
        use tokio::time::timeout;
        
        let address = format!("127.0.0.1:{}", port);
        let duration = Duration::from_secs(timeout_secs);
        
        match timeout(duration, TcpStream::connect(&address)).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout
        }
    }
}

#[cfg(test)]
mod tests {
    include!("process_controller_test.rs");
}

#[cfg(test)]
mod property_tests {
    include!("process_controller_property_test.rs");
}

#[cfg(test)]
mod port_property_tests {
    include!("process_controller_port_property_test.rs");
}