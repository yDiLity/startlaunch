import React from "react";
import { AlertTriangle, Shield, AlertCircle, XCircle } from "lucide-react";

interface SecurityWarning {
  level: "Low" | "Medium" | "High" | "Critical";
  message: string;
  suggestion: string | null;
}

interface SecurityWarningsProps {
  warnings: SecurityWarning[];
  onTrustRepository?: () => void;
  isTrusted?: boolean;
}

const SecurityWarnings: React.FC<SecurityWarningsProps> = ({ 
  warnings, 
  onTrustRepository,
  isTrusted = false 
}) => {
  if (warnings.length === 0 && isTrusted) {
    return (
      <div className="bg-green-900/30 border border-green-700 rounded-lg p-4">
        <div className="flex items-start space-x-3">
          <Shield className="w-5 h-5 text-green-400 flex-shrink-0 mt-0.5" />
          <div>
            <h4 className="font-semibold text-green-400 mb-1">
              Доверенный репозиторий
            </h4>
            <p className="text-green-300 text-sm">
              Этот репозиторий находится в списке доверенных. Проект будет запущен без дополнительных проверок.
            </p>
          </div>
        </div>
      </div>
    );
  }

  if (warnings.length === 0) {
    return null;
  }

  const getIcon = (level: string) => {
    switch (level) {
      case "Critical":
        return <XCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />;
      case "High":
        return <AlertTriangle className="w-5 h-5 text-orange-500 flex-shrink-0 mt-0.5" />;
      case "Medium":
        return <AlertCircle className="w-5 h-5 text-yellow-500 flex-shrink-0 mt-0.5" />;
      default:
        return <AlertCircle className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />;
    }
  };

  const getBgColor = (level: string) => {
    switch (level) {
      case "Critical":
        return "bg-red-900/50 border-red-700";
      case "High":
        return "bg-orange-900/50 border-orange-700";
      case "Medium":
        return "bg-yellow-900/50 border-yellow-700";
      default:
        return "bg-blue-900/50 border-blue-700";
    }
  };

  const getTextColor = (level: string) => {
    switch (level) {
      case "Critical":
        return "text-red-400";
      case "High":
        return "text-orange-400";
      case "Medium":
        return "text-yellow-400";
      default:
        return "text-blue-400";
    }
  };

  const criticalWarnings = warnings.filter(w => w.level === "Critical");
  const hasCriticalWarnings = criticalWarnings.length > 0;

  return (
    <div className="space-y-4">
      <div className={`border rounded-lg p-4 ${getBgColor(warnings[0].level)}`}>
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-start space-x-3">
            {getIcon(warnings[0].level)}
            <div>
              <h4 className={`font-semibold ${getTextColor(warnings[0].level)} mb-1`}>
                {hasCriticalWarnings ? "КРИТИЧЕСКАЯ УГРОЗА БЕЗОПАСНОСТИ" : "Предупреждения безопасности"}
              </h4>
              <p className="text-gray-300 text-sm">
                Обнаружено {warnings.length} {warnings.length === 1 ? "предупреждение" : "предупреждений"}
              </p>
            </div>
          </div>
        </div>

        <div className="space-y-3">
          {warnings.map((warning, index) => (
            <div key={index} className="bg-gray-800/50 rounded p-3">
              <div className="flex items-start space-x-2 mb-2">
                <span className={`text-xs font-semibold px-2 py-1 rounded ${
                  warning.level === "Critical" ? "bg-red-700 text-red-100" :
                  warning.level === "High" ? "bg-orange-700 text-orange-100" :
                  warning.level === "Medium" ? "bg-yellow-700 text-yellow-100" :
                  "bg-blue-700 text-blue-100"
                }`}>
                  {warning.level === "Critical" ? "КРИТИЧНО" :
                   warning.level === "High" ? "ВЫСОКИЙ" :
                   warning.level === "Medium" ? "СРЕДНИЙ" : "НИЗКИЙ"}
                </span>
              </div>
              <p className="text-white text-sm mb-2">{warning.message}</p>
              {warning.suggestion && (
                <p className="text-gray-400 text-sm italic">
                  💡 {warning.suggestion}
                </p>
              )}
            </div>
          ))}
        </div>

        {hasCriticalWarnings && (
          <div className="mt-4 p-3 bg-red-950/50 border border-red-800 rounded">
            <p className="text-red-300 text-sm font-semibold">
              ⚠️ НЕ ЗАПУСКАЙТЕ ЭТОТ ПРОЕКТ! Обнаружены критические угрозы безопасности, которые могут повредить вашу систему.
            </p>
          </div>
        )}

        {!hasCriticalWarnings && onTrustRepository && !isTrusted && (
          <div className="mt-4 flex items-center justify-between p-3 bg-gray-800/50 rounded">
            <div className="flex-1">
              <p className="text-gray-300 text-sm">
                Вы доверяете этому репозиторию? Добавьте его в список доверенных, чтобы пропустить проверки безопасности в будущем.
              </p>
            </div>
            <button
              onClick={onTrustRepository}
              className="ml-4 bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors whitespace-nowrap"
            >
              Добавить в доверенные
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default SecurityWarnings;
