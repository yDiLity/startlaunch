import { invoke } from "@tauri-apps/api/tauri";
import type React from "react";
import { useCallback, useEffect, useState } from "react";
import "./Settings.css";

interface AppSettings {
	default_isolation_mode: "Sandbox" | "Direct";
	snapshots_path: string;
	theme: "Light" | "Dark" | "System";
	auto_cleanup: boolean;
	max_snapshot_age_days: number;
	enable_logging: boolean;
}

interface SettingsProps {
	onClose?: () => void;
}

const Settings: React.FC<SettingsProps> = ({ onClose }) => {
	const [settings, setSettings] = useState<AppSettings | null>(null);
	const [loading, setLoading] = useState(true);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [successMessage, setSuccessMessage] = useState<string | null>(null);

	const loadSettings = useCallback(async () => {
		try {
			setLoading(true);
			setError(null);
			const loadedSettings = await invoke<AppSettings>("get_settings");
			setSettings(loadedSettings);
		} catch (err) {
			setError(`Ошибка загрузки настроек: ${err}`);
			console.error("Ошибка загрузки настроек:", err);
		} finally {
			setLoading(false);
		}
	}, []);

	const applyTheme = useCallback((theme: "Light" | "Dark" | "System") => {
		// Требование 9.3: Применение темы ко всему интерфейсу
		const root = document.documentElement;

		if (theme === "System") {
			const prefersDark = window.matchMedia(
				"(prefers-color-scheme: dark)",
			).matches;
			root.setAttribute("data-theme", prefersDark ? "dark" : "light");
		} else {
			root.setAttribute("data-theme", theme.toLowerCase());
		}
	}, []);

	// Загрузка настроек при монтировании компонента
	useEffect(() => {
		void loadSettings();
	}, [loadSettings]);

	// Применение темы к документу
	useEffect(() => {
		if (settings) {
			applyTheme(settings.theme);
		}
	}, [applyTheme, settings]);

	const handleSave = async () => {
		if (!settings) return;

		try {
			setSaving(true);
			setError(null);
			setSuccessMessage(null);

			// Требование 9.5: Сохранение настроек в конфигурационный файл
			await invoke("update_settings", { settings });

			setSuccessMessage("Настройки успешно сохранены!");

			// Скрываем сообщение через 3 секунды
			setTimeout(() => setSuccessMessage(null), 3000);
		} catch (err) {
			setError(`Ошибка сохранения настроек: ${err}`);
			console.error("Ошибка сохранения настроек:", err);
		} finally {
			setSaving(false);
		}
	};

	const handleReset = async () => {
		if (
			!confirm(
				"Вы уверены, что хотите сбросить все настройки к значениям по умолчанию?",
			)
		) {
			return;
		}

		try {
			setSaving(true);
			setError(null);
			await invoke("reset_settings_to_defaults");
			await loadSettings();
			setSuccessMessage("Настройки сброшены к значениям по умолчанию");
			setTimeout(() => setSuccessMessage(null), 3000);
		} catch (err) {
			setError(`Ошибка сброса настроек: ${err}`);
			console.error("Ошибка сброса настроек:", err);
		} finally {
			setSaving(false);
		}
	};

	const handleBrowseSnapshotsPath = async () => {
		try {
			const { open } = await import("@tauri-apps/api/dialog");
			const selected = await open({
				directory: true,
				multiple: false,
				title: "Выберите директорию для снимков",
			});

			if (selected && typeof selected === "string") {
				setSettings((prev) =>
					prev ? { ...prev, snapshots_path: selected } : null,
				);
			}
		} catch (err) {
			console.error("Ошибка выбора директории:", err);
		}
	};

	if (loading) {
		return (
			<div className="settings-container">
				<div className="settings-loading">
					<div className="spinner"></div>
					<p>Загрузка настроек...</p>
				</div>
			</div>
		);
	}

	if (!settings) {
		return (
			<div className="settings-container">
				<div className="settings-error">
					<p>Не удалось загрузить настройки</p>
					<button type="button" onClick={loadSettings}>
						Попробовать снова
					</button>
				</div>
			</div>
		);
	}

	return (
		<div className="settings-container">
			<div className="settings-header">
				<h2>Настройки приложения</h2>
				{onClose && (
					<button
						type="button"
						className="close-button"
						onClick={onClose}
						aria-label="Закрыть"
					>
						✕
					</button>
				)}
			</div>

			{error && (
				<div className="alert alert-error">
					<span className="alert-icon">⚠️</span>
					<span>{error}</span>
				</div>
			)}

			{successMessage && (
				<div className="alert alert-success">
					<span className="alert-icon">✓</span>
					<span>{successMessage}</span>
				</div>
			)}

			<div className="settings-content">
				{/* Требование 9.1: Настройки режима изоляции */}
				<div className="settings-section">
					<h3>Безопасность</h3>

					<div className="setting-item">
						<label htmlFor="isolation-mode">
							<span className="setting-label">Режим изоляции по умолчанию</span>
							<span className="setting-description">
								Определяет, как будут запускаться проекты
							</span>
						</label>
						<select
							id="isolation-mode"
							value={settings.default_isolation_mode}
							onChange={(e) =>
								setSettings({
									...settings,
									default_isolation_mode: e.target.value as
										| "Sandbox"
										| "Direct",
								})
							}
						>
							<option value="Sandbox">
								Песочница (Docker) - Рекомендуется
							</option>
							<option value="Direct">
								Прямой режим (Виртуальное окружение)
							</option>
						</select>
					</div>
				</div>

				{/* Требование 9.2: Настройки путей */}
				<div className="settings-section">
					<h3>Хранилище</h3>

					<div className="setting-item">
						<label htmlFor="snapshots-path">
							<span className="setting-label">Путь для сохранения снимков</span>
							<span className="setting-description">
								Директория для хранения снимков проектов
							</span>
						</label>
						<div className="path-input-group">
							<input
								id="snapshots-path"
								type="text"
								value={settings.snapshots_path}
								onChange={(e) =>
									setSettings({
										...settings,
										snapshots_path: e.target.value,
									})
								}
								placeholder="/путь/к/директории"
							/>
							<button
								type="button"
								onClick={handleBrowseSnapshotsPath}
								className="browse-button"
							>
								Обзор...
							</button>
						</div>
					</div>

					<div className="setting-item">
						<label htmlFor="max-age">
							<span className="setting-label">
								Максимальный возраст снимков (дни)
							</span>
							<span className="setting-description">
								Снимки старше указанного возраста будут автоматически удалены
							</span>
						</label>
						<input
							id="max-age"
							type="number"
							min="1"
							max="365"
							value={settings.max_snapshot_age_days}
							onChange={(e) =>
								setSettings({
									...settings,
									max_snapshot_age_days: parseInt(e.target.value, 10) || 30,
								})
							}
						/>
					</div>
				</div>

				{/* Требование 9.3: Настройки темы оформления */}
				<div className="settings-section">
					<h3>Внешний вид</h3>

					<div className="setting-item">
						<label htmlFor="theme">
							<span className="setting-label">Тема оформления</span>
							<span className="setting-description">
								Цветовая схема интерфейса приложения
							</span>
						</label>
						<select
							id="theme"
							value={settings.theme}
							onChange={(e) =>
								setSettings({
									...settings,
									theme: e.target.value as "Light" | "Dark" | "System",
								})
							}
						>
							<option value="Light">Светлая</option>
							<option value="Dark">Темная</option>
							<option value="System">Системная</option>
						</select>
					</div>
				</div>

				{/* Требование 9.4: Настройки автоочистки */}
				<div className="settings-section">
					<h3>Обслуживание</h3>

					<div className="setting-item">
						<label className="checkbox-label">
							<input
								type="checkbox"
								checked={settings.auto_cleanup}
								onChange={(e) =>
									setSettings({
										...settings,
										auto_cleanup: e.target.checked,
									})
								}
							/>
							<span className="setting-label">Автоматическая очистка</span>
						</label>
						<span className="setting-description">
							Автоматически удалять временные файлы после остановки проектов
						</span>
					</div>

					<div className="setting-item">
						<label className="checkbox-label">
							<input
								type="checkbox"
								checked={settings.enable_logging}
								onChange={(e) =>
									setSettings({
										...settings,
										enable_logging: e.target.checked,
									})
								}
							/>
							<span className="setting-label">Включить логирование</span>
						</label>
						<span className="setting-description">
							Сохранять подробные логи работы приложения
						</span>
					</div>
				</div>
			</div>

			<div className="settings-footer">
				<button
					type="button"
					className="button button-secondary"
					onClick={handleReset}
					disabled={saving}
				>
					Сбросить к умолчаниям
				</button>
				<div className="button-group">
					{onClose && (
						<button
							type="button"
							className="button button-outline"
							onClick={onClose}
							disabled={saving}
						>
							Отмена
						</button>
					)}
					<button
						type="button"
						className="button button-primary"
						onClick={handleSave}
						disabled={saving}
					>
						{saving ? "Сохранение..." : "Сохранить"}
					</button>
				</div>
			</div>
		</div>
	);
};

export default Settings;
