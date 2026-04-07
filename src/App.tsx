import { invoke } from "@tauri-apps/api/tauri";
import {
	ExternalLink,
	Github,
	History,
	Loader2,
	Play,
	RotateCcw,
	Settings,
	Square,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import ProcessWindow, {
	type LogEntry,
	type ProcessStage,
} from "./components/ProcessWindow";
import ProjectManager from "./components/ProjectManager";
import SecurityWarnings from "./components/SecurityWarnings";
import SettingsModal from "./components/Settings";

interface SecurityWarning {
	id: string;
	level: "Low" | "Medium" | "High" | "Critical";
	message: string;
	suggestion: string | null;
}

interface ProjectDependency {
	name: string;
	version?: string | null;
	dev?: boolean;
}

interface ProjectConfigFile {
	path?: string;
	file_type?: string;
}

interface UserFriendlyError {
	user_friendly_message?: string;
}

interface ProjectInfo {
	stack: unknown;
	entry_command: string | null;
	dependencies: ProjectDependency[];
	config_files: ProjectConfigFile[];
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

interface AnalysisResult {
	project_id: string;
	project_info: ProjectInfo;
}

interface BackendProcessLog {
	timestamp: string;
	level: string;
	message: string;
}

interface SavedProject {
	id: string;
	github_url: string;
	owner: string;
	repo_name: string;
	local_path: string;
	detected_stack: string;
	trust_level: string;
	created_at: string;
	last_run_at: string | null;
	tags: string;
}

const createLogEntry = (
	level: "info" | "warning" | "error" | "success",
	message: string,
): LogEntry => ({
	id: crypto.randomUUID(),
	timestamp: new Date().toISOString(),
	level,
	message,
});

const normalizeLogLevel = (level: string): LogEntry["level"] => {
	switch (level.toLowerCase()) {
		case "error":
			return "error";
		case "warning":
			return "warning";
		case "success":
			return "success";
		default:
			return "info";
	}
};

const normalizeProcessLogs = (logs: BackendProcessLog[]): LogEntry[] =>
	logs.map((log, index) => ({
		id: `${log.timestamp}-${log.level}-${log.message}-${index}`,
		timestamp: log.timestamp,
		level: normalizeLogLevel(log.level),
		message: log.message,
	}));

const mergeLogEntries = (
	existingLogs: LogEntry[],
	incomingLogs: LogEntry[],
): LogEntry[] => {
	const seenIds = new Set(existingLogs.map((entry) => entry.id));
	const nextLogs = [...existingLogs];

	for (const entry of incomingLogs) {
		if (!seenIds.has(entry.id)) {
			seenIds.add(entry.id);
			nextLogs.push(entry);
		}
	}

	return nextLogs;
};

const getStageFromStatus = (status: ProjectStatus): ProcessStage => {
	const normalizedStatus = status.status.toLowerCase();

	if (!status.running) {
		return normalizedStatus.includes("fail") ? "failed" : "stopped";
	}

	if (normalizedStatus.includes("stopp")) {
		return "stopping";
	}

	if (
		normalizedStatus.includes("start") ||
		normalizedStatus.includes("prepar")
	) {
		return "starting";
	}

	return "running";
};

const getProgressFromStage = (stage: ProcessStage): number => {
	const progressMap: Record<ProcessStage, number> = {
		cloning: 15,
		analyzing: 55,
		installing: 65,
		configuring: 75,
		starting: 85,
		running: 100,
		stopping: 90,
		stopped: 100,
		failed: 100,
	};

	return progressMap[stage];
};

function App() {
	const [url, setUrl] = useState("");
	const [isAnalyzing, setIsAnalyzing] = useState(false);
	const [projectInfo, setProjectInfo] = useState<ProjectInfo | null>(null);
	const [projectId, setProjectId] = useState<string | null>(null);
	const [projectStatus, setProjectStatus] = useState<ProjectStatus | null>(
		null,
	);
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
	const [processErrorSuggestion, setProcessErrorSuggestion] = useState<
		string | null
	>(null);

	// State for manual command override
	const [commandOverride, setCommandOverride] = useState<string>("");

	const syncProjectRuntime = useCallback(async (targetProjectId: string) => {
		try {
			const status = await invoke<ProjectStatus>("get_project_status", {
				project_id: targetProjectId,
			});

			setProjectStatus(status);

			const nextStage = getStageFromStatus(status);
			setProcessStage(nextStage);
			setProcessProgress(getProgressFromStage(nextStage));

			if (status.running) {
				const rawLogs = await invoke<BackendProcessLog[]>("get_process_logs", {
					project_id: targetProjectId,
				});
				const normalizedLogs = normalizeProcessLogs(rawLogs);
				setProcessLogs((prev) => mergeLogEntries(prev, normalizedLogs));
			}
		} catch (err) {
			console.error("Ошибка синхронизации состояния проекта:", err);
		}
	}, []);

	const loadSavedProjectContext = useCallback(
		async (targetProjectId: string) => {
			try {
				const projects = await invoke<SavedProject[]>("get_project_history");
				const savedProject = projects.find(
					(project) => project.id === targetProjectId,
				);

				if (savedProject) {
					setCurrentRepoUrl(savedProject.github_url);
					setUrl(savedProject.github_url);
				}
			} catch (err) {
				console.error("Ошибка загрузки сохранённого проекта:", err);
			}
		},
		[],
	);

	// Опрос статуса процесса и логов
	useEffect(() => {
		if (projectId && projectStatus?.running) {
			void syncProjectRuntime(projectId);

			const interval = setInterval(() => {
				void syncProjectRuntime(projectId);
			}, 3000);

			return () => clearInterval(interval);
		}
	}, [projectId, projectStatus?.running, syncProjectRuntime]);

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
		setProcessStage("cloning");
		setProcessProgress(getProgressFromStage("cloning"));
		setProcessLogs([
			createLogEntry("info", `Начинаем анализ репозитория: ${url}`),
			createLogEntry("info", "Клонирование и анализ выполняются в backend..."),
		]);
		setProcessError(null);
		setProcessErrorSuggestion(null);

		try {
			const result = await invoke<AnalysisResult>("analyze_repository", {
				url,
			});

			setProcessStage("analyzing");
			setProcessProgress(100);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry("success", "Репозиторий успешно проанализирован"),
			]);

			setProjectInfo(result.project_info);
			setProjectId(result.project_id);

			setProcessLogs((prev) => [
				...prev,
				createLogEntry(
					"success",
					`Проект успешно проанализирован. Обнаружен стек: ${JSON.stringify(result.project_info.stack)}`,
				),
			]);

			// Проверяем доверие
			try {
				const trusted = await invoke<boolean>("is_trusted_repository", {
					repo_url: url,
				});
				setIsTrusted(trusted);

				if (trusted) {
					setProcessLogs((prev) => [
						...prev,
						createLogEntry("info", "Репозиторий находится в списке доверенных"),
					]);
				}
			} catch (err) {
				console.error("Ошибка проверки доверия:", err);
			}

			setTimeout(() => setShowProcessWindow(false), 2000);
		} catch (err: unknown) {
			const typedError = err as UserFriendlyError;
			setError(
				typedError.user_friendly_message ||
					"Произошла ошибка при анализе репозитория",
			);
			setProcessStage("failed");
			setProcessError(
				typedError.user_friendly_message ||
					"Произошла ошибка при анализе репозитория",
			);
			setProcessErrorSuggestion(
				"Проверьте правильность URL и доступность репозитория",
			);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry(
					"error",
					typedError.user_friendly_message || "Ошибка анализа репозитория",
				),
			]);
		} finally {
			setIsAnalyzing(false);
		}
	};

	const handleTrustRepository = async () => {
		if (!currentRepoUrl) return;

		try {
			await invoke("add_trusted_repository", { repo_url: currentRepoUrl });
			setIsTrusted(true);
		} catch (err: unknown) {
			const typedError = err as UserFriendlyError;
			setError(
				typedError.user_friendly_message ||
					"Ошибка при добавлении в доверенные",
			);
		}
	};

	const handleStart = async (targetProjectId = projectId) => {
		if (!targetProjectId) return;

		setIsStarting(true);
		setError(null);
		setStartMessage(null);
		setProjectId(targetProjectId);

		setShowProcessWindow(true);
		setProcessStage("starting");
		setProcessProgress(getProgressFromStage("starting"));
		setProcessLogs([
			createLogEntry("info", "Подготавливаем запуск проекта..."),
		]);
		setProcessError(null);
		setProcessErrorSuggestion(null);

		try {
			setProcessLogs((prev) => [
				...prev,
				createLogEntry("info", "Команда запуска передана в backend..."),
			]);

			const message = await invoke<string>("start_project", {
				project_id: targetProjectId,
				command_override: commandOverride || null,
			});

			setProcessStage("starting");
			setProcessProgress(95);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry("success", "Backend завершил запуск проекта"),
				createLogEntry("info", message),
			]);

			setStartMessage(message);
			await syncProjectRuntime(targetProjectId);
		} catch (err: unknown) {
			const typedError = err as UserFriendlyError;
			setError(
				typedError.user_friendly_message ||
					"Произошла ошибка при запуске проекта",
			);
			setProcessStage("failed");
			setProcessError(
				typedError.user_friendly_message ||
					"Произошла ошибка при запуске проекта",
			);
			setProcessErrorSuggestion(
				"Проверьте логи для получения дополнительной информации",
			);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry(
					"error",
					typedError.user_friendly_message || "Ошибка запуска проекта",
				),
			]);
		} finally {
			setIsStarting(false);
		}
	};

	const handleStop = async () => {
		if (!projectId) return;

		setIsStopping(true);
		setError(null);
		setShowProcessWindow(true);
		setProcessStage("stopping");
		setProcessProgress(getProgressFromStage("stopping"));
		setProcessLogs((prev) => [
			...prev,
			createLogEntry("info", "Останавливаем проект..."),
		]);

		try {
			await invoke("stop_project", { project_id: projectId });
			setProjectStatus({ running: false, status: "stopped" });
			setStartMessage(null);
			setProcessStage("stopped");
			setProcessProgress(100);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry("success", "Проект остановлен"),
			]);
		} catch (err: unknown) {
			const typedError = err as UserFriendlyError;
			setError(
				typedError.user_friendly_message ||
					"Произошла ошибка при остановке проекта",
			);
		} finally {
			setIsStopping(false);
		}
	};

	const handleRestart = async () => {
		if (!projectId) return;

		setIsStopping(true);
		setError(null);
		setShowProcessWindow(true);
		setProcessStage("starting");
		setProcessProgress(getProgressFromStage("starting"));
		setProcessLogs((prev) => [
			...prev,
			createLogEntry("info", "Перезапуск проекта..."),
		]);

		try {
			await invoke("restart_project", {
				project_id: projectId,
				command_override: commandOverride || null,
			});

			setProcessStage("starting");
			setProcessProgress(95);
			setProcessLogs((prev) => [
				...prev,
				createLogEntry("success", "Проект успешно перезапущен!"),
				createLogEntry(
					"info",
					"Ожидаем подтверждение статуса после перезапуска...",
				),
			]);
			setStartMessage("Проект успешно перезапущен");
			await syncProjectRuntime(projectId);
		} catch (err: unknown) {
			const typedError = err as UserFriendlyError;
			setError(
				typedError.user_friendly_message || "Ошибка при перезапуске проекта",
			);
			setProcessStage("failed");
			setProcessError(typedError.user_friendly_message || "Ошибка перезапуска");
			setProcessLogs((prev) => [
				...prev,
				createLogEntry(
					"error",
					typedError.user_friendly_message || "Ошибка перезапуска",
				),
			]);
		} finally {
			setIsStopping(false);
		}
	};

	const openInBrowser = () => {
		if (projectStatus?.detected_port) {
			window.open(`http://localhost:${projectStatus.detected_port}`, "_blank");
		}
	};

	const handleLaunchProjectFromManager = async (launchProjectId: string) => {
		await loadSavedProjectContext(launchProjectId);
		setProjectId(launchProjectId);
		setProjectInfo(null);
		setProjectStatus(null);
		setError(null);
		await handleStart(launchProjectId);
	};

	const handleStartClick = () => {
		void handleStart();
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
							type="button"
							onClick={() => setShowProjectManager(true)}
							className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
							title="История проектов"
						>
							<History className="w-5 h-5" />
						</button>
						<button
							type="button"
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
							type="button"
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
								<div
									className={`w-3 h-3 rounded-full ${projectStatus.running ? "bg-green-400" : "bg-red-400"}`}
								></div>
								<span
									className={`font-medium ${projectStatus.running ? "text-green-400" : "text-red-400"}`}
								>
									{projectStatus.running ? "Запущен" : "Остановлен"}
								</span>
							</div>
						</div>

						<div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
							<div>
								<span className="text-gray-400 text-sm">Статус:</span>
								<div className="text-white font-mono">
									{projectStatus.status}
								</div>
							</div>

							{projectStatus.environment_type && (
								<div>
									<span className="text-gray-400 text-sm">Окружение:</span>
									<div className="text-white font-mono">
										{projectStatus.environment_type === "docker"
											? "Docker"
											: "Прямое"}
									</div>
								</div>
							)}

							{projectStatus.detected_port && (
								<div>
									<span className="text-gray-400 text-sm">Порт:</span>
									<div className="text-white font-mono">
										{projectStatus.detected_port}
									</div>
								</div>
							)}

							{projectStatus.process_id && (
								<div>
									<span className="text-gray-400 text-sm">ID процесса:</span>
									<div className="text-white font-mono text-sm">
										{projectStatus.process_id}
									</div>
								</div>
							)}
						</div>

						<div className="flex items-center space-x-3">
							{projectStatus.running ? (
								<>
									<button
										type="button"
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
										type="button"
										onClick={handleRestart}
										disabled={isStopping || isStarting}
										className="bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 px-4 py-2 rounded-lg font-medium transition-colors flex items-center space-x-2"
									>
										<RotateCcw className="w-4 h-4" />
										<span>Перезапустить</span>
									</button>

									{projectStatus.detected_port && (
										<button
											type="button"
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
									type="button"
									onClick={handleStartClick}
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
										type="button"
										onClick={handleStartClick}
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
									<h4 className="font-semibold text-gray-300 mb-2">
										Стек технологий
									</h4>
									<div className="bg-gray-700 rounded p-3">
										<span className="text-blue-400 font-mono">
											{JSON.stringify(projectInfo.stack)}
										</span>
									</div>
								</div>

								{/* Entry Command with Override */}
								<div>
									<h4 className="font-semibold text-gray-300 mb-2">
										Команда запуска
									</h4>
									<div className="bg-gray-700 rounded p-3">
										<input
											type="text"
											value={commandOverride}
											onChange={(e) => setCommandOverride(e.target.value)}
											placeholder={projectInfo.entry_command || "Не определена"}
											className="w-full bg-gray-600 border border-gray-500 rounded px-3 py-2 text-green-400 font-mono text-sm focus:outline-none focus:border-blue-400"
										/>
										{commandOverride && (
											<p className="text-xs text-gray-400 mt-1">
												Будет использована эта команда вместо автоопределённой
											</p>
										)}
									</div>
								</div>

								{/* Entry Command */}
								<div>
									<h4 className="font-semibold text-gray-300 mb-2">
										Команда запуска
									</h4>
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
												{projectInfo.dependencies.slice(0, 10).map((dep) => (
													<li key={dep.name} className="text-sm">
														<span className="text-yellow-400">{dep.name}</span>
														{dep.version && (
															<span className="text-gray-400">
																{" "}
																@ {dep.version}
															</span>
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
											<span className="text-gray-400">
												Зависимости не найдены
											</span>
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
							Вставьте ссылку на GitHub репозиторий для автоматического анализа
							и запуска
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
						isClosable={
							processStage === "running" ||
							processStage === "failed" ||
							processStage === "stopped"
						}
					/>
				</div>
			)}
		</div>
	);
}

export default App;
