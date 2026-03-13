const express = require('express');
const app = express();
const port = process.env.PORT || 3000;

app.get('/', (req, res) => {
  res.send(`
    <html>
      <head>
        <title>AutoLaunch Test Project</title>
        <style>
          body { font-family: Arial, sans-serif; margin: 40px; background: #f0f0f0; }
          .container { max-width: 600px; margin: 0 auto; background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
          h1 { color: #333; text-align: center; }
          .success { background: #d4edda; color: #155724; padding: 15px; border-radius: 4px; margin: 20px 0; }
          .info { background: #d1ecf1; color: #0c5460; padding: 15px; border-radius: 4px; margin: 20px 0; }
        </style>
      </head>
      <body>
        <div class="container">
          <h1>🚀 AutoLaunch Test Project</h1>
          <div class="success">
            ✅ Проект успешно запущен через AutoLaunch!
          </div>
          <div class="info">
            <strong>Информация о сервере:</strong><br>
            • Порт: ${port}<br>
            • Node.js версия: ${process.version}<br>
            • Время запуска: ${new Date().toLocaleString('ru-RU')}
          </div>
          <p>Этот тестовый проект демонстрирует работу AutoLaunch - автоматического анализа и запуска GitHub проектов.</p>
        </div>
      </body>
    </html>
  `);
});

app.listen(port, () => {
  console.log(\`🚀 Сервер запущен на порту \${port}\`);
  console.log(\`📱 Откройте http://localhost:\${port} в браузере\`);
});