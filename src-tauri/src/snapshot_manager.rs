use crate::error::{AutoLaunchError, Result};
use crate::models::{ProjectSnapshot, SnapshotMetadata, EnvironmentType, ProjectInfo, Dependency};
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::Utc;

/// Менеджер снимков проектов
/// Отвечает за сохранение, загрузку и удаление снимков проектов
pub struct SnapshotManager {
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new() -> Result<Self> {
        let snapshots_dir = dirs::data_dir()
            .ok_or_else(|| AutoLaunchError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Не удалось найти директорию данных"
            )))?
            .join("autolaunch")
            .join("snapshots");

        // Создаем директорию для снимков если её нет
        if !snapshots_dir.exists() {
            tracing::info!("Создание директории для снимков: {:?}", snapshots_dir);
            fs::create_dir_all(&snapshots_dir)?;
        }

        Ok(Self { snapshots_dir })
    }

    /// Создает снимок проекта (Требование 7.2)
    /// Сохраняет все зависимости, конфигурацию и метаданные
    pub async fn create_snapshot(
        &self,
        project_id: &str,
        project_path: &Path,
        project_info: &ProjectInfo,
        environment_type: EnvironmentType,
        ports: Vec<u16>,
        environment_variables: Vec<(String, String)>,
    ) -> Result<ProjectSnapshot> {
        tracing::info!("Создание снимка для проекта: {}", project_id);

        let snapshot_id = Uuid::new_v4().to_string();
        let snapshot_path = self.snapshots_dir.join(&snapshot_id);

        // Создаем директорию для снимка
        fs::create_dir_all(&snapshot_path)?;
        tracing::debug!("Создана директория снимка: {:?}", snapshot_path);

        // Копируем файлы проекта
        tracing::info!("Копирование файлов проекта в снимок");
        self.copy_project_files(project_path, &snapshot_path)?;

        // Создаем метаданные (Требование 7.4)
        let metadata = SnapshotMetadata {
            entry_command: project_info.entry_command.clone(),
            ports,
            environment_variables,
            dependencies: project_info.dependencies.clone(),
            tech_stack: project_info.stack.to_string(),
        };

        // Сохраняем метаданные в JSON файл
        let metadata_path = snapshot_path.join("snapshot_metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;
        tracing::debug!("Метаданные сохранены в: {:?}", metadata_path);

        // Вычисляем размер снимка
        let size_bytes = self.calculate_directory_size(&snapshot_path)?;
        tracing::info!("Размер снимка: {} байт", size_bytes);

        let snapshot = ProjectSnapshot {
            id: snapshot_id,
            project_id: project_id.to_string(),
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            environment_type: match environment_type {
                EnvironmentType::Docker => "docker".to_string(),
                EnvironmentType::Direct => "direct".to_string(),
            },
            metadata: serde_json::to_string(&metadata)?,
            created_at: Utc::now().to_rfc3339(),
            size_bytes,
        };

        tracing::info!("Снимок успешно создан: {}", snapshot.id);
        Ok(snapshot)
    }

    /// Загружает снимок проекта для быстрого запуска (Требование 7.3)
    pub async fn load_snapshot(&self, snapshot_id: &str) -> Result<(PathBuf, SnapshotMetadata)> {
        tracing::info!("Загрузка снимка: {}", snapshot_id);

        let snapshot_path = self.snapshots_dir.join(snapshot_id);
        if !snapshot_path.exists() {
            return Err(AutoLaunchError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Снимок не найден: {}", snapshot_id)
            )));
        }

        // Загружаем метаданные
        let metadata_path = snapshot_path.join("snapshot_metadata.json");
        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata: SnapshotMetadata = serde_json::from_str(&metadata_json)?;

        tracing::info!("Снимок успешно загружен: {}", snapshot_id);
        Ok((snapshot_path, metadata))
    }

    /// Удаляет снимок проекта (Требование 7.5)
    /// Полностью очищает все связанные файлы
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        tracing::info!("Удаление снимка: {}", snapshot_id);

        let snapshot_path = self.snapshots_dir.join(snapshot_id);
        if !snapshot_path.exists() {
            tracing::warn!("Снимок не найден: {}", snapshot_id);
            return Ok(());
        }

        // Полностью удаляем директорию снимка
        fs::remove_dir_all(&snapshot_path)?;
        tracing::info!("Снимок успешно удален: {}", snapshot_id);

        Ok(())
    }

    /// Получает список всех снимков для проекта
    pub async fn list_snapshots(&self, project_id: &str) -> Result<Vec<String>> {
        tracing::debug!("Получение списка снимков для проекта: {}", project_id);

        let mut snapshots = Vec::new();
        
        if !self.snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let metadata_path = path.join("snapshot_metadata.json");
                if metadata_path.exists() {
                    snapshots.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }

        Ok(snapshots)
    }

    /// Копирует файлы проекта в директорию снимка
    fn copy_project_files(&self, source: &Path, destination: &Path) -> Result<()> {
        tracing::debug!("Копирование файлов из {:?} в {:?}", source, destination);

        // Список директорий и файлов, которые нужно исключить
        let exclude_patterns = vec![
            ".git",
            "node_modules",
            "target",
            "__pycache__",
            ".venv",
            "venv",
            "dist",
            "build",
            ".cache",
        ];

        self.copy_directory_recursive(source, destination, &exclude_patterns)?;
        Ok(())
    }

    /// Рекурсивно копирует директорию с исключениями
    fn copy_directory_recursive(
        &self,
        source: &Path,
        destination: &Path,
        exclude_patterns: &[&str],
    ) -> Result<()> {
        if !destination.exists() {
            fs::create_dir_all(destination)?;
        }

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Пропускаем исключенные директории и файлы
            if exclude_patterns.iter().any(|pattern| file_name_str.contains(pattern)) {
                tracing::trace!("Пропуск исключенного файла/директории: {:?}", path);
                continue;
            }

            let dest_path = destination.join(&file_name);

            if path.is_dir() {
                self.copy_directory_recursive(&path, &dest_path, exclude_patterns)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }

        Ok(())
    }

    /// Вычисляет размер директории в байтах
    fn calculate_directory_size(&self, path: &Path) -> Result<i64> {
        let mut total_size = 0i64;

        if path.is_file() {
            return Ok(fs::metadata(path)?.len() as i64);
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                total_size += self.calculate_directory_size(&entry_path)?;
            } else {
                total_size += fs::metadata(&entry_path)?.len() as i64;
            }
        }

        Ok(total_size)
    }

    /// Очищает старые снимки (опциональная функциональность)
    pub async fn cleanup_old_snapshots(&self, max_age_days: u64) -> Result<Vec<String>> {
        tracing::info!("Очистка снимков старше {} дней", max_age_days);

        let mut deleted_snapshots = Vec::new();
        let max_age = chrono::Duration::days(max_age_days as i64);
        let now = Utc::now();

        if !self.snapshots_dir.exists() {
            return Ok(deleted_snapshots);
        }

        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let metadata_path = path.join("snapshot_metadata.json");
                if metadata_path.exists() {
                    let file_metadata = fs::metadata(&metadata_path)?;
                    if let Ok(modified) = file_metadata.modified() {
                        let modified_datetime: DateTime<Utc> = modified.into();
                        if now.signed_duration_since(modified_datetime) > max_age {
                            let snapshot_id = entry.file_name().to_string_lossy().to_string();
                            self.delete_snapshot(&snapshot_id).await?;
                            deleted_snapshots.push(snapshot_id);
                        }
                    }
                }
            }
        }

        tracing::info!("Удалено {} старых снимков", deleted_snapshots.len());
        Ok(deleted_snapshots)
    }
}

#[cfg(test)]
#[path = "snapshot_manager_test.rs"]
mod tests;

#[cfg(test)]
mod property_tests {
    include!("snapshot_manager_property_test.rs");
}
