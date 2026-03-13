import React, { useState, useEffect, useRef } from "react";
import { X, ChevronDown, ChevronUp, CheckCircle, XCircle, Loader2, AlertCircle } from "lucide-react";

// Требование 5.1: Отображение текущего этапа в прогресс-баре
export type ProcessStage = 
  | "cloning"
  | "analyzing"
  | "installing"
  | "configuring"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "failed";

export interface LogEntry {
  timestamp: string;
  level: "info" | "warning" | "error" | "success";
  message: string;
}

interface ProcessWindowProps {
  projectName: string;
  currentStage: ProcessStage;
  progress: number; // 0-100
  logs: LogEntry[];
  error?: string | null;
  errorSuggestion?: string | null;
  onClose?: () => void;
  onStop?: () => void;
  isClosable?: boolean;
}

const ProcessWindow: React.FC<ProcessWindowProps> = ({
  projectName,
  currentStage,
  progress,
  logs,
  error,
  errorSuggestion,
  onClose,
  onStop,
  isClosable = true,
}) => {
  const [showDetails, setShowDetails] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const logsContainerRef = useRef<HTMLDivElement>(null);

  // Требование 5.2: Показывать логи установки в реальном времени
  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll]);

  // Отслеживание ручной прокрутки
  const handleScroll = () => {
    if (!logsContainerRef.current) return;
    
    const { scrollTop, scrollHeight, clientHeight } = logsContainerRef.current;
    const isAtBottom = Math.abs(scrollHeight - clientHeight - scrollTop) < 10;
    
    setAutoScroll(isAtBottom);
  };

  const getStageLabel = (stage: ProcessStage): string => {
    const labels: Record<ProcessStage, string> = {
      cloning: "Клонирование репозитория",
      analyzing: "Анализ проекта",
      installing: "Установка зависимостей",
      configuring: "Настройка окружения",
      starting: "Запуск приложения",
      running: "Приложение запущено",
      stopping: "Остановка приложения",
      stopped: "Приложение остановлено",
      failed: "Ошибка выполнения",
    };
    return labels[stage];
  };

  const getStageIcon = (stage: ProcessStage) => {
    if (stage === "failed") {
      return <XCircle className="w-5 h-5 text-red-400" />;
    }
    if (stage === "running" || stage === "stopped") {
      return <CheckCircle className="w-5 h-5 text-green-400" />;
    }
    return <Loader2 className="w-5 h-5 text-blue-400 animate-spin" />;
  };

  const getProgressColor = (): string => {
    if (error || currentStage === "failed") return "bg-red-500";
    if (currentStage === "running") return "bg-green-500";
    return "bg-blue-500";
  };

  const getLogLevelColor = (level: LogEntry["level"]): string => {
    const colors = {
      info: "text-gray-300",
      warning: "text-yellow-400",
      error: "text-red-400",
      success: "text-green-400",
    };
    return colors[level];
  };

  const getLogLevelIcon = (level: LogEntry["level"]) => {
    switch (level) {
      case "error":
        return "❌";
      case "warning":
        return "⚠️";
      case "success":
        return "✅";
      default:
        return "ℹ️";
    }
  };

  const formatTimestamp = (timestamp: string): string => {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString("ru-RU", {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return timestamp;
    }
  };

  return (
    <div className="bg-gray-800 rounded-lg shadow-xl overflow-hidden">
      {/* Header */}
      <div className="bg-gray-900 px-4 py-3 flex items-center justify-between border-b border-gray-700">
        <div className="flex items-center space-x-3">
          {getStageIcon(currentStage)}
          <div>
            <h3 className="font-semibold text-white">{projectName}</h3>
            <p className="text-sm text-gray-400">{getStageLabel(currentStage)}</p>
          </div>
        </div>
        <div className="flex items-center space-x-2">
          {currentStage !== "stopped" && currentStage !== "failed" && onStop && (
            <button
              onClick={onStop}
              className="px-3 py-1 bg-red-600 hover:bg-red-700 rounded text-sm transition-colors"
            >
              Остановить
            </button>
          )}
          {isClosable && onClose && (
            <button
              onClick={onClose}
              className="p-1 hover:bg-gray-700 rounded transition-colors"
              aria-label="Закрыть"
            >
              <X className="w-5 h-5" />
            </button>
          )}
        </div>
      </div>

      {/* Progress Bar - Требование 5.1 */}
      <div className="px-4 py-3 bg-gray-850">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-gray-300">Прогресс</span>
          <span className="text-sm font-semibold text-white">{progress}%</span>
        </div>
        <div className="w-full bg-gray-700 rounded-full h-2 overflow-hidden">
          <div
            className={`h-full transition-all duration-300 ${getProgressColor()}`}
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>

      {/* Требование 5.3: Понятное сообщение об ошибке с предложениями решения */}
      {error && (
        <div className="mx-4 mt-3 bg-red-900/50 border border-red-700 rounded-lg p-3">
          <div className="flex items-start space-x-2">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <h4 className="font-semibold text-red-400 mb-1">Ошибка</h4>
              <p className="text-red-300 text-sm mb-2">{error}</p>
              {errorSuggestion && (
                <div className="bg-red-950/50 rounded p-2 mt-2">
                  <p className="text-red-200 text-sm">
                    <strong>💡 Предложение:</strong> {errorSuggestion}
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Требование 5.5: Статус "Готово" с зеленой галочкой */}
      {currentStage === "running" && !error && (
        <div className="mx-4 mt-3 bg-green-900/50 border border-green-700 rounded-lg p-3">
          <div className="flex items-center space-x-2">
            <CheckCircle className="w-5 h-5 text-green-400" />
            <span className="text-green-300 font-semibold">Готово! Приложение успешно запущено</span>
          </div>
        </div>
      )}

      {/* Logs Section - Требование 5.2 и 5.4 */}
      <div className="px-4 py-3">
        <button
          onClick={() => setShowDetails(!showDetails)}
          className="flex items-center justify-between w-full text-left text-sm font-semibold text-gray-300 hover:text-white transition-colors"
        >
          <span>Логи выполнения ({logs.length})</span>
          {showDetails ? (
            <ChevronUp className="w-4 h-4" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
        </button>

        {showDetails && (
          <div className="mt-3">
            <div
              ref={logsContainerRef}
              onScroll={handleScroll}
              className="bg-gray-900 rounded border border-gray-700 p-3 max-h-64 overflow-y-auto font-mono text-xs"
            >
              {logs.length === 0 ? (
                <p className="text-gray-500 text-center py-4">Логи пока отсутствуют</p>
              ) : (
                <div className="space-y-1">
                  {logs.map((log, index) => (
                    <div key={index} className="flex items-start space-x-2">
                      <span className="text-gray-500 flex-shrink-0">
                        {formatTimestamp(log.timestamp)}
                      </span>
                      <span className="flex-shrink-0">{getLogLevelIcon(log.level)}</span>
                      <span className={`flex-1 ${getLogLevelColor(log.level)}`}>
                        {log.message}
                      </span>
                    </div>
                  ))}
                  <div ref={logsEndRef} />
                </div>
              )}
            </div>
            
            {!autoScroll && (
              <button
                onClick={() => {
                  setAutoScroll(true);
                  logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
                }}
                className="mt-2 text-xs text-blue-400 hover:text-blue-300 transition-colors"
              >
                ↓ Прокрутить вниз
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default ProcessWindow;
