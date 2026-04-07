import { invoke } from "@tauri-apps/api/tauri";
import { Calendar, Folder, Play, Search, Tag, Trash2, X } from "lucide-react";
import type React from "react";
import { useCallback, useEffect, useState } from "react";

interface Project {
	id: string;
	github_url: string;
	owner: string;
	repo_name: string;
	local_path: string;
	detected_stack: string;
	trust_level: string;
	created_at: string;
	last_run_at: string | null;
	tags: string; // JSON array
}

interface ProjectManagerProps {
	onClose: () => void;
	onLaunchProject: (projectId: string) => void;
}

const ProjectManager: React.FC<ProjectManagerProps> = ({
	onClose,
	onLaunchProject,
}) => {
	const [projects, setProjects] = useState<Project[]>([]);
	const [filteredProjects, setFilteredProjects] = useState<Project[]>([]);
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedTags, setSelectedTags] = useState<string[]>([]);
	const [allTags, setAllTags] = useState<string[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [editingTags, setEditingTags] = useState<string | null>(null);
	const [newTag, setNewTag] = useState("");

	useEffect(() => {
		loadProjects();
		loadAllTags();
	}, []);

	useEffect(() => {
		filterProjects();
	}, [projects, searchQuery, selectedTags]);

	const loadProjects = useCallback(async () => {
		try {
			setIsLoading(true);
			const result = await invoke<Project[]>("get_project_history");
			setProjects(result);
			setFilteredProjects(result);
		} catch (err: unknown) {
			const error = err as { user_friendly_message?: string };
			setError(error.user_friendly_message || "Ошибка загрузки проектов");
		} finally {
			setIsLoading(false);
		}
	}, []);

	const loadAllTags = useCallback(async () => {
		try {
			const tags = await invoke<string[]>("get_all_tags");
			setAllTags(tags);
		} catch (err) {
			console.error("Ошибка загрузки тегов:", err);
		}
	}, []);

	const filterProjects = useCallback(() => {
		let filtered = projects;

		// Фильтрация по поисковому запросу
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			filtered = filtered.filter(
				(p) =>
					p.repo_name.toLowerCase().includes(query) ||
					p.owner.toLowerCase().includes(query) ||
					p.detected_stack.toLowerCase().includes(query),
			);
		}

		// Фильтрация по тегам
		if (selectedTags.length > 0) {
			filtered = filtered.filter((p) => {
				const projectTags: string[] = JSON.parse(p.tags || "[]");
				return selectedTags.some((tag) => projectTags.includes(tag));
			});
		}

		setFilteredProjects(filtered);
	}, [projects, searchQuery, selectedTags]);

	const handleDeleteProject = async (projectId: string) => {
		if (!confirm("Вы уверены, что хотите удалить этот проект?")) {
			return;
		}

		try {
			await invoke("delete_project", { project_id: projectId });
			await loadProjects();
			await loadAllTags();
		} catch (err: unknown) {
			const error = err as { user_friendly_message?: string };
			setError(error.user_friendly_message || "Ошибка удаления проекта");
		}
	};

	const handleLaunchProject = async (projectId: string) => {
		try {
			await invoke("update_project_last_run", { project_id: projectId });
			onLaunchProject(projectId);
			onClose();
		} catch (err: unknown) {
			const error = err as { user_friendly_message?: string };
			setError(error.user_friendly_message || "Ошибка запуска проекта");
		}
	};

	const handleAddTag = async (projectId: string, tag: string) => {
		if (!tag.trim()) return;

		try {
			const currentTags: string[] = JSON.parse(
				projects.find((p) => p.id === projectId)?.tags || "[]",
			);

			if (currentTags.includes(tag.trim())) {
				return; // Тег уже существует
			}

			const updatedTags = [...currentTags, tag.trim()];
			await invoke("update_project_tags", {
				project_id: projectId,
				tags: updatedTags,
			});

			await loadProjects();
			await loadAllTags();
			setNewTag("");
		} catch (err: any) {
			setError(err.user_friendly_message || "Ошибка добавления тега");
		}
	};

	const handleRemoveTag = async (projectId: string, tagToRemove: string) => {
		try {
			const currentTags: string[] = JSON.parse(
				projects.find((p) => p.id === projectId)?.tags || "[]",
			);
			const updatedTags = currentTags.filter((t) => t !== tagToRemove);

			await invoke("update_project_tags", {
				project_id: projectId,
				tags: updatedTags,
			});

			await loadProjects();
			await loadAllTags();
		} catch (err: any) {
			setError(err.user_friendly_message || "Ошибка удаления тега");
		}
	};

	const toggleTagFilter = (tag: string) => {
		setSelectedTags((prev) =>
			prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
		);
	};

	const formatDate = (dateStr: string | null) => {
		if (!dateStr) return "Никогда";
		const date = new Date(dateStr);
		return date.toLocaleDateString("ru-RU", {
			year: "numeric",
			month: "short",
			day: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		});
	};

	const getProjectTags = (project: Project): string[] => {
		try {
			return JSON.parse(project.tags || "[]");
		} catch {
			return [];
		}
	};

	return (
		<div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div className="bg-gray-800 rounded-lg w-full max-w-6xl max-h-[90vh] overflow-hidden flex flex-col">
				{/* Header */}
				<div className="p-6 border-b border-gray-700">
					<div className="flex items-center justify-between mb-4">
						<h2 className="text-2xl font-bold">Менеджер проектов</h2>
						<button
							type="button"
							onClick={onClose}
							className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
						>
							<X className="w-5 h-5" />
						</button>
					</div>

					{/* Search and Filters */}
					<div className="space-y-4">
						<div className="relative">
							<Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
							<input
								type="text"
								value={searchQuery}
								onChange={(e) => setSearchQuery(e.target.value)}
								placeholder="Поиск по имени, владельцу или стеку..."
								className="w-full bg-gray-700 border border-gray-600 rounded-lg pl-10 pr-4 py-2 focus:outline-none focus:border-blue-400"
							/>
						</div>

						{/* Tag Filters */}
						{allTags.length > 0 && (
							<div className="flex flex-wrap gap-2">
								<span className="text-sm text-gray-400 flex items-center">
									<Tag className="w-4 h-4 mr-1" />
									Фильтр по тегам:
								</span>
								{allTags.map((tag) => (
									<button
										type="button"
										key={tag}
										onClick={() => toggleTagFilter(tag)}
										className={`px-3 py-1 rounded-full text-sm transition-colors ${
											selectedTags.includes(tag)
												? "bg-blue-600 text-white"
												: "bg-gray-700 text-gray-300 hover:bg-gray-600"
										}`}
									>
										{tag}
									</button>
								))}
								{selectedTags.length > 0 && (
									<button
										type="button"
										onClick={() => setSelectedTags([])}
										className="px-3 py-1 rounded-full text-sm bg-red-600 text-white hover:bg-red-700"
									>
										Сбросить
									</button>
								)}
							</div>
						)}
					</div>
				</div>

				{/* Error Display */}
				{error && (
					<div className="mx-6 mt-4 bg-red-900/50 border border-red-700 rounded-lg p-3">
						<p className="text-red-300 text-sm">{error}</p>
					</div>
				)}

				{/* Projects List */}
				<div className="flex-1 overflow-y-auto p-6">
					{isLoading ? (
						<div className="text-center py-12">
							<div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-400 mx-auto"></div>
							<p className="text-gray-400 mt-4">Загрузка проектов...</p>
						</div>
					) : filteredProjects.length === 0 ? (
						<div className="text-center py-12">
							<Folder className="w-16 h-16 text-gray-600 mx-auto mb-4" />
							<p className="text-gray-400">
								{searchQuery || selectedTags.length > 0
									? "Проекты не найдены"
									: "У вас пока нет проектов"}
							</p>
						</div>
					) : (
						<div className="grid grid-cols-1 gap-4">
							{filteredProjects.map((project) => (
								<div
									key={project.id}
									className="bg-gray-700 rounded-lg p-4 hover:bg-gray-650 transition-colors"
								>
									<div className="flex items-start justify-between">
										<div className="flex-1">
											<div className="flex items-center space-x-3 mb-2">
												<h3 className="text-lg font-semibold text-white">
													{project.owner}/{project.repo_name}
												</h3>
												<span className="px-2 py-1 bg-blue-900/50 text-blue-300 text-xs rounded">
													{project.detected_stack}
												</span>
											</div>

											<div className="flex items-center space-x-4 text-sm text-gray-400 mb-3">
												<div className="flex items-center">
													<Calendar className="w-4 h-4 mr-1" />
													Создан: {formatDate(project.created_at)}
												</div>
												{project.last_run_at && (
													<div className="flex items-center">
														<Play className="w-4 h-4 mr-1" />
														Запущен: {formatDate(project.last_run_at)}
													</div>
												)}
											</div>

											{/* Tags */}
											<div className="flex flex-wrap gap-2 items-center">
												{getProjectTags(project).map((tag) => (
													<span
														key={tag}
														className="px-2 py-1 bg-gray-600 text-gray-200 text-xs rounded flex items-center space-x-1"
													>
														<Tag className="w-3 h-3" />
														<span>{tag}</span>
														{editingTags === project.id && (
															<button
																type="button"
																onClick={() => handleRemoveTag(project.id, tag)}
																className="ml-1 hover:text-red-400"
															>
																<X className="w-3 h-3" />
															</button>
														)}
													</span>
												))}

												{editingTags === project.id ? (
													<div className="flex items-center space-x-2">
														<input
															type="text"
															value={newTag}
															onChange={(e) => setNewTag(e.target.value)}
															onKeyPress={(e) => {
																if (e.key === "Enter") {
																	handleAddTag(project.id, newTag);
																}
															}}
															placeholder="Новый тег..."
															className="px-2 py-1 bg-gray-600 border border-gray-500 rounded text-xs focus:outline-none focus:border-blue-400"
														/>
														<button
															type="button"
															onClick={() => {
																handleAddTag(project.id, newTag);
															}}
															className="px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-xs"
														>
															Добавить
														</button>
														<button
															type="button"
															onClick={() => {
																setEditingTags(null);
																setNewTag("");
															}}
															className="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs"
														>
															Готово
														</button>
													</div>
												) : (
													<button
														type="button"
														onClick={() => setEditingTags(project.id)}
														className="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs flex items-center space-x-1"
													>
														<Tag className="w-3 h-3" />
														<span>Редактировать теги</span>
													</button>
												)}
											</div>
										</div>

										{/* Actions */}
										<div className="flex items-center space-x-2 ml-4">
											<button
												type="button"
												onClick={() => handleLaunchProject(project.id)}
												className="p-2 bg-green-600 hover:bg-green-700 rounded-lg transition-colors"
												title="Запустить проект"
											>
												<Play className="w-5 h-5" />
											</button>
											<button
												type="button"
												onClick={() => handleDeleteProject(project.id)}
												className="p-2 bg-red-600 hover:bg-red-700 rounded-lg transition-colors"
												title="Удалить проект"
											>
												<Trash2 className="w-5 h-5" />
											</button>
										</div>
									</div>
								</div>
							))}
						</div>
					)}
				</div>

				{/* Footer */}
				<div className="p-4 border-t border-gray-700 bg-gray-750">
					<div className="flex items-center justify-between text-sm text-gray-400">
						<span>
							Показано проектов: {filteredProjects.length} из {projects.length}
						</span>
						<button
							type="button"
							onClick={onClose}
							className="px-4 py-2 bg-gray-600 hover:bg-gray-500 rounded-lg transition-colors"
						>
							Закрыть
						</button>
					</div>
				</div>
			</div>
		</div>
	);
};

export default ProjectManager;
