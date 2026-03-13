import React, { useState, useEffect } from "react";
// import { invoke } from "@tauri-apps/api/tauri";
import { Github, Play, Settings, History, Loader2, Square, RotateCcw, ExternalLink } from "lucide-react";
import SecurityWarnings from "./components/SecurityWarnings";
import ProjectManager from "./components/ProjectManager";
import ProcessWindow, { ProcessStage, LogEntry } from "./components/ProcessWindow";
import SettingsModal from "./components/Settings";

interface SecurityWarning {
  level: "Low" | "Medium" | "High" | "Critical";
  message: string;
  suggestion: string | null;
}

interface ProjectInfo {
  stack: any;
  entry_command: string | null;
  dependencies: any[];
  config_files: any[];
  security_warnings: SecurityWarning[];
}

interface ProjectStatus {
  running: boolean;
  status: string;
  process_id?: string;
  container_id?: string;
  ports?: number[];
  detected_port?: number;
  environment_type?: string;
}

// Мок функция для демонстрации без Tauri
const mockInvoke = async (command: string, args: any): Promise<any> => {
  await new Promise(resolve => setTimeout(resolve, 1000)); // Имитация задержки
  
  if (command === "analyze_repository") {
    return {
      stack: { NodeJs: { version: "18.0.0" } },
      entry_command: "npm start",
      dependencies: [
        { name: "react", version: "^18.2.0", dev: false },
        { name: "typescript", version: "^5.2.2", dev: true },
        { name: "vite", version: "^4.5.0", dev: true }
      ],
      config_files: [],
      security_warnings: []
    };
  }
  
  if (command === "start_project") {
    return "Проект успешно запущен! ID процесса: mock-123. Приложение доступно на http://localhost:3000";
  }
  
  if (command === "stop_project") {
    return null;
  }
  
  if (command === "get_project_status") {
    return {
      running: Math.random() > 0.5,
      status: "Running",
      process_id: "mock-123",
      ports: [3000],
      detected_port: 3000,
      environment_type: "direct"
    };
  }
  
  throw new Error("Команда не найдена");
};

function App() {
  const [url, setUrl] = useState("");
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [projectInfo, setProjectInfo] = useState<ProjectInfo | null>(null);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [projectStatus, setProjectStatus] = useState<ProjectStatus | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [startMessage, setStartMessage] = useState<string | null>(null);
  const [isTrusted, setIsTrusted] = useState(false);
  const [currentRepoUrl, setCurrentRepoUrl] = useState<string | null>(null);
  const [showProjectManager, setShowProjectManager] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  
  // Process Window state
  const [showProcessWindow, setShowProcessWindow] = useState(false);
  const [processStage, setProcessStage] = useState<ProcessStage>("cloning");
  const [processProgress, setProcessProgress] = useState(0);
  const [processLogs, setProcessLogs] = useState<LogEntry[]>([]);
  const [processError, setProcessError] = useState<string | null>(null);
  const [processErrorSuggestion, setProcessErrorSuggestion] = useState<string | null>(null);

  // Опрос статуса проекта
  useEffect(() => {
    if (projectId && projectStatus?.running) {
      const interval = setInterval(async () => {
        try {
          const status = await mockInvoke("get_project_status", { project_id: projectId });
          setProjectStatus(status);
        } catch (err) {
          console.error("Ошибка получения статуса:", err);
        }
      }, 3000);

      return () => clearInterval(interval);
    }
  }, [projectId, projectStatus?.running]);

  const handleAnalyze = async () => {
    if (!url.trim()) return;

    setIsAnalyzing(true);
    setError(null);
    setProjectInfo(null);
    setProjectId(null);
    setProjectStatus(null);
    setStartMessage(null);
    setIsTrusted(false);
    setCurrentRepoUrl(url);
    
    // Показываем окно процесса
    setShowProcessWindow(true);
    setProcessStage("analyzing");
    setProcessProgress(10);
    setProcessLogs([{
      timestamp: new Date().toISOString(),
      level: "info",
      message: `Начинаем анализ репозитория: ${url}`
    }]);
    setProcessError(null);
    setProcessErrorSuggestion(null);

    try {
      // Симуляция прогресса анализа
      setProcessProgress(30);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Клонирование репозитория..."
      }]);
      
      const result = await mockInvoke("analyze_repository", { url });
      
      setProcessProgress(70);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "success",
        message: "Репозиторий успешно клонирован"
      }, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Анализ структуры проекта..."
      }]);
      
      setProjectInfo(result);
      setProjectId("mock-project-id");
      
      setProcessProgress(100);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "success",
        message: `Проект успешно проанализирован. Обнаружен стек: ${JSON.stringify(result.stack)}`
      }]);
      
      // Проверяем доверие
      try {
        const trusted = await mockInvoke("is_trusted_repository", { repo_url: url });
        setIsTrusted(trusted);
        
        if (trusted) {
          setProcessLogs(prev => [...prev, {
            timestamp: new Date().toISOString(),
            level: "info",
            message: "Репозиторий находится в списке доверенных"
          }]);
        }
      } catch (err) {
        console.error("Ошибка проверки доверия:", err);
      }
      
      // Скрываем окно процесса после успешного анализа
      setTimeout(() => setShowProcessWindow(false), 2000);
      
    } catch (err: any) {
      setError(err.user_friendly_message || "Произошла ошибка при анализе репозитория");
      setProcessStage("failed");
      setProcessError(err.user_friendly_message || "Произошла ошибка при анализе репозитория");
      setProcessErrorSuggestion("Проверьте правильность URL и доступность репозитория");
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "error",
        message: err.user_friendly_message || "Ошибка анализа репозитория"
      }]);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleTrustRepository = async () => {
    if (!currentRepoUrl) return;

    try {
      await mockInvoke("add_trusted_repository", { repo_url: currentRepoUrl });
      setIsTrusted(true);
    } catch (err: any) {
      setError(err.user_friendly_message || "Ошибка при добавлении в доверенные");
    }
  };

  const handleStart = async () => {
    if (!projectId) return;

    setIsStarting(true);
    setError(null);
    setStartMessage(null);
    
    // Показываем окно процесса
    setShowProcessWindow(true);
    setProcessStage("installing");
    setProcessProgress(0);
    setProcessLogs([{
      timestamp: new Date().toISOString(),
      level: "info",
      message: "Начинаем установку зависимостей..."
    }]);
    setProcessError(null);
    setProcessErrorSuggestion(null);

    try {
      // Симуляция установки зависимостей
      setProcessProgress(20);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Создание изолированного окружения..."
      }]);
      
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProcessProgress(40);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Установка зависимостей npm..."
      }, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "npm install react@^18.2.0"
      }, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "npm install typescript@^5.2.2"
      }]);
      
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProcessStage("configuring");
      setProcessProgress(60);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "success",
        message: "Зависимости успешно установлены"
      }, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Настройка окружения..."
      }]);
      
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProcessStage("starting");
      setProcessProgress(80);
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: "Запуск приложения..."
      }]);
      
      const message = await mockInvoke("start_project", { project_id: projectId });
      
      setProcessProgress(100);
      setProcessStage("running");
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "success",
        message: "Приложение успешно запущено!"
      }, {
        timestamp: new Date().toISOString(),
        level: "info",
        message: message
      }]);
      
      setStartMessage(message);
      
      // Получаем статус после запуска
      setTimeout(async () => {
        try {
          const status = await mockInvoke("get_project_status", { project_id: projectId });
          setProjectStatus(status);
        } catch (err) {
          console.error("Ошибка получения статуса:", err);
        }
      }, 2000);
      
    } catch (err: any) {
      setError(err.user_friendly_message || "Произошла ошибка при запуске проекта");
      setProcessStage("failed");
      setProcessError(err.user_friendly_message || "Произошла ошибка при запуске проекта");
      setProcessErrorSuggestion("Проверьте логи для получения дополнительной информации");
      setProcessLogs(prev => [...prev, {
        timestamp: new Date().toISOString(),
        level: "error",
        message: err.user_friendly_message || "Ошибка запуска проекта"
      }]);
    } finally {
      setIsStarting(false);
    }
  };

  const handleStop = async () => {
    if (!projectId) return;

    setIsStopping(true);
    setError(null);

    try {
      await mockInvoke("stop_project", { project_id: projectId });
      setProjectStatus({ running: false, status: "stopped" });
      setStartMessage(null);
    } catch (err: any) {
      setError(err.user_friendly_message || "Произошла ошибка при остановке проекта");
    } finally {
      setIsStopping(false);
    }
  };

  const handleRestart = async () => {
    await handleStop();
    setTimeout(() => {
      handleStart();
    }, 1000);
  };

  const openInBrowser = () => {
    if (projectStatus?.detected_port) {
      window.open(`http://localhost:${projectStatus.detected_port}`, '_blank');
    }
  };

  const handleLaunchProjectFromManager = (launchProjectId: string) => {
    setProjectId(launchProjectId);
    // В реальной реализации здесь будет загрузка информации о проекте
    // и автоматический запуск
    handleStart();
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700 p-4">
        <div className="flex items-center justify-between max-w-6xl mx-auto">
          <div className="flex items-center space-x-2">
            <Github className="w-8 h-8 text-blue-400" />
            <h1 className="text-2xl font-bold">AutoLaunch</h1>
          </div>
          <div className="flex items-center space-x-4">
            <button 
              onClick={() => setShowProjectManager(true)}
              className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
              title="История проектов"
            >
              <History className="w-5 h-5" />
            </button>
            <button 
              onClick={() => setShowSettings(true)}
              className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
              title="Настройки"
            >
              <Settings className="w-5 h-5" />
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-4xl mx-auto p-6">
        {/* URL Input Section */}
        <div className="bg-gray-800 rounded-lg p-6 mb-6">
          <h2 className="text-xl font-semibold mb-4">Запуск GitHub проекта</h2>
          <div className="flex space-x-4">
            <input
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="Вставьте ссылку на GitHub репозиторий или owner/repo..."
              className="flex-1 bg-gray-700 border border-gray-600 rounded-lg px-4 py-2 focus:outline-none focus:border-blue-400"
              onKeyPress={(e) => e.key === "Enter" && handleAnalyze()}
            />
            <button
              onClick={handleAnalyze}
              disabled={isAnalyzing || !url.trim()}
              className="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 px-6 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
            >
              {isAnalyzing ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>Анализ...</span>
                </>
              ) : (
                <>
                  <Play className="w-4 h-4" />
                  <span>Анализировать</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Success Message */}
        {startMessage && (
          <div className="bg-green-900/50 border border-green-700 rounded-lg p-4 mb-6">
            <h3 className="font-semibold text-green-400 mb-2">Успех</h3>
            <p className="text-green-300">{startMessage}</p>
          </div>
        )}

        {/* Project Status */}
        {projectStatus && (
          <div className="bg-gray-800 rounded-lg p-6 mb-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-xl font-semibold">Статус проекта</h3>
              <div className="flex items-center space-x-2">
                <div className={`w-3 h-3 rounded-full ${projectStatus.running ? 'bg-green-400' : 'bg-red-400'}`}></div>
                <span className={`font-medium ${projectStatus.running ? 'text-green-400' : 'text-red-400'}`}>
                  {projectStatus.running ? 'Запущен' : 'Остановлен'}
                </span>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
              <div>
                <span className="text-gray-400 text-sm">Статус:</span>
                <div className="text-white font-mono">{projectStatus.status}</div>
              </div>
              
              {projectStatus.environment_type && (
                <div>
                  <span className="text-gray-400 text-sm">Окружение:</span>
                  <div className="text-white font-mono">
                    {projectStatus.environment_type === 'docker' ? 'Docker' : 'Прямое'}
                  </div>
                </div>
              )}

              {projectStatus.detected_port && (
                <div>
                  <span className="text-gray-400 text-sm">Порт:</span>
                  <div className="text-white font-mono">{projectStatus.detected_port}</div>
                </div>
              )}

              {projectStatus.process_id && (
                <div>
                  <span className="text-gray-400 text-sm">ID процесса:</span>
                  <div className="text-white font-mono text-sm">{projectStatus.process_id}</div>
                </div>
              )}
            </div>

            <div className="flex items-center space-x-3">
              {projectStatus.running ? (
                <>
                  <button
                    onClick={handleStop}
                    disabled={isStopping}
                    className="bg-red-600 hover:bg-red-700 disabled:bg-gray-600 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
                  >
                    {isStopping ? (
                      <>
                        <Loader2 className="w-4 h-4 animate-spin" />
                        <span>Остановка...</span>
                      </>
                    ) : (
                      <>
                        <Square className="w-4 h-4" />
                        <span>Остановить</span>
                      </>
                    )}
                  </button>
                  
                  <button
                    onClick={handleRestart}
                    disabled={isStopping || isStarting}
                    className="bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
                  >
                    <RotateCcw className="w-4 h-4" />
                    <span>Перезапустить</span>
                  </button>

                  {projectStatus.detected_port && (
                    <button
                      onClick={openInBrowser}
                      className="bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
                    >
                      <ExternalLink className="w-4 h-4" />
                      <span>Открыть в браузере</span>
                    </button>
                  )}
                </>
              ) : (
                <button
                  onClick={handleStart}
                  disabled={isStarting}
                  className="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
                >
                  {isStarting ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>Запуск...</span>
                    </>
                  ) : (
                    <>
                      <Play className="w-4 h-4" />
                      <span>Запустить</span>
                    </>
                  )}
                </button>
              )}
            </div>
          </div>
        )}

        {/* Error Display */}
        {error && (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-4 mb-6">
            <h3 className="font-semibold text-red-400 mb-2">Ошибка</h3>
            <p className="text-red-300">{error}</p>
          </div>
        )}
        {/* Project Info Display */}
        {projectInfo && (
          <div className="space-y-6">
            {/* Security Warnings */}
            {(projectInfo.security_warnings.length > 0 || isTrusted) && (
              <SecurityWarnings 
                warnings={projectInfo.security_warnings}
                onTrustRepository={handleTrustRepository}
                isTrusted={isTrusted}
              />
            )}

            <div className="bg-gray-800 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-xl font-semibold">Информация о проекте</h3>
              {!projectStatus?.running && (
                <button
                  onClick={handleStart}
                  disabled={isStarting}
                  className="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
                >
                  {isStarting ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>Запуск...</span>
                    </>
                  ) : (
                    <>
                      <Play className="w-4 h-4" />
                      <span>Запустить</span>
                    </>
                  )}
                </button>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* Stack Info */}
              <div>
                <h4 className="font-semibold text-gray-300 mb-2">Стек технологий</h4>
                <div className="bg-gray-700 rounded p-3">
                  <span className="text-blue-400 font-mono">
                    {JSON.stringify(projectInfo.stack)}
                  </span>
                </div>
              </div>

              {/* Entry Command */}
              <div>
                <h4 className="font-semibold text-gray-300 mb-2">Команда запуска</h4>
                <div className="bg-gray-700 rounded p-3">
                  <span className="text-green-400 font-mono">
                    {projectInfo.entry_command || "Не определена"}
                  </span>
                </div>
              </div>

              {/* Dependencies */}
              <div className="md:col-span-2">
                <h4 className="font-semibold text-gray-300 mb-2">
                  Зависимости ({projectInfo.dependencies.length})
                </h4>
                <div className="bg-gray-700 rounded p-3 max-h-40 overflow-y-auto">
                  {projectInfo.dependencies.length > 0 ? (
                    <ul className="space-y-1">
                      {projectInfo.dependencies.slice(0, 10).map((dep, index) => (
                        <li key={index} className="text-sm">
                          <span className="text-yellow-400">{dep.name}</span>
                          {dep.version && (
                            <span className="text-gray-400"> @ {dep.version}</span>
                          )}
                          {dep.dev && (
                            <span className="text-blue-400 ml-2">(dev)</span>
                          )}
                        </li>
                      ))}
                      {projectInfo.dependencies.length > 10 && (
                        <li className="text-gray-400 text-sm">
                          ... и еще {projectInfo.dependencies.length - 10}
                        </li>
                      )}
                    </ul>
                  ) : (
                    <span className="text-gray-400">Зависимости не найдены</span>
                  )}
                </div>
              </div>
            </div>
          </div>
          </div>
        )}

        {/* Welcome Message */}
        {!projectInfo && !error && !isAnalyzing && (
          <div className="text-center py-12">
            <Github className="w-16 h-16 text-gray-600 mx-auto mb-4" />
            <h2 className="text-2xl font-semibold text-gray-400 mb-2">
              Добро пожаловать в AutoLaunch
            </h2>
            <p className="text-gray-500">
              Вставьте ссылку на GitHub репозиторий для автоматического анализа и запуска
            </p>
          </div>
        )}
      </main>

      {/* Project Manager Modal */}
      {showProjectManager && (
        <ProjectManager
          onClose={() => setShowProjectManager(false)}
          onLaunchProject={handleLaunchProjectFromManager}
        />
      )}

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg w-full max-w-4xl max-h-[90vh] overflow-hidden">
            <SettingsModal onClose={() => setShowSettings(false)} />
          </div>
        </div>
      )}

      {/* Process Window */}
      {showProcessWindow && (
        <div className="fixed bottom-4 right-4 w-full max-w-2xl z-40">
          <ProcessWindow
            projectName={currentRepoUrl || "Проект"}
            currentStage={processStage}
            progress={processProgress}
            logs={processLogs}
            error={processError}
            errorSuggestion={processErrorSuggestion}
            onClose={() => setShowProcessWindow(false)}
            onStop={handleStop}
            isClosable={processStage === "running" || processStage === "failed" || processStage === "stopped"}
          />
        </div>
      )}
    </div>
  );
}

export default App;