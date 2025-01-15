use rinja::Template;

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device_width, initial_scale=1.0">
    <title>Error Encontrado</title>
 <style>
        :root {
            --primary-color: #3b82f6;
            --error-color: #fee2e2;
            --border-color: #fecaca;
            --text-color: #450a0a;
        }

        body {
            font-family: system-ui, -apple-system, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background-color: #f9fafb;
        }

        .error_container {
            background-color: white;
            border-radius: 8px;
            box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
            padding: 2rem;
            max-width: 32rem;
            width: 90%;
            text-align: center;
        }

        .error_icon {
            background-color: var(--error-color);
            border-radius: 50%;
            width: 48px;
            height: 48px;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 1rem;
        }

        .error_icon svg {
            color: #dc2626;
            width: 24px;
            height: 24px;
        }

        .error_title {
            color: #111827;
            font-size: 1.5rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }

        .error_timestamp {
            color: #6b7280;
            font-size: 0.875rem;
            margin-bottom: 1rem;
        }

        .error_message {
            color: #374151;
            margin-bottom: 1rem;
        }

        .error_details {
            background-color: var(--error-color);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            padding: 1rem;
            margin-bottom: 1.5rem;
            text-align: left;
            color: var(--text-color);
            font-family: monospace;
            font-size: 0.875rem;
            white-space: pre-wrap;
            overflow-x: auto;
        }

        .button_group {
            display: flex;
            gap: 0.75rem;
            justify-content: center;
        }

        .button {
            padding: 0.5rem 1rem;
            border-radius: 6px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
            font-size: 0.875rem;
        }

        .primary_button {
            background-color: var(--primary-color);
            color: white;
            border: none;
        }

        .primary_button:hover {
            background-color: #2563eb;
        }

        .secondary_button {
            background-color: white;
            color: #374151;
            border: 1px solid #d1d5db;
        }

        .secondary_button:hover {
            background-color: #f9fafb;
        }
    </style>
</head>
<body>
    <div class="error_container">
        <div class="error_icon">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
        </div>
        <h1 class="error_title">Error Encontrado</h1>
        <div class="error_timestamp">{{ date }}</div>
        <p class="error_message">Ha ocurrido un error. Por favor, intente nuevamente o busque ayuda.</p>
        <div class="error_details"> {{ err }} </div>
        
        <div class="button_group">
            <a href="" class="button primary_button" role="button">Intentar Nuevamente</a>
            <a href="/" class="button secondary_button " role="button">Volver al Inicio</a>
        </div>
    </div>
</body>
</html>
"#
)]
pub struct Fallback {
    pub err: String,
    pub date: String,
}
