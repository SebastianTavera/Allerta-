# Faro — detección de situaciones de riesgo en la universidad

Aplicación de escritorio en **Rust** para que los estudiantes reporten:

- Zonas inseguras
- Daños en las instalaciones
- Emergencias
- Acoso
- Lugares que necesitan iluminación
- Otras situaciones de riesgo

Cada reporte se guarda en **SQLite** y se envía por **correo electrónico** a
`joanromana.st@academia.umb.edu.co`.

## Interfaz

- Estilo "vidrio líquido" monocromático, con bordes redondeados en todo.
- Panel lateral izquierdo de vidrio; en ventanas angostas se contrae a iconos.
- El puntero es una **luz**: un núcleo brillante con un halo que lo sigue con
  inercia y se refleja en los bordes y superficies del vidrio. Al hacer clic
  emite una onda.
- Animaciones con **resortes físicos** (inercia y un rebote leve): la píldora
  de navegación se estira al desplazarse, las tarjetas entran escalonadas y los
  contadores suben con rebote.

## Requisitos para compilar

1. Instala Rust: <https://rustup.rs>
2. Solo en Linux, librerías de desarrollo gráfico (Debian/Ubuntu/Raspberry Pi OS):
   ```bash
   sudo apt install build-essential libxkbcommon-dev libgl1-mesa-dev libwayland-dev
   ```
   Windows y macOS no necesitan nada extra.

## Ejecutar

```bash
cargo run --release
```

El binario queda en `target/release/faro` (`faro.exe` en Windows). Es un único
ejecutable: SQLite va incluido y el TLS del correo está en Rust puro, así que
no requiere instalar nada más en el equipo donde se use.

## Configurar el correo

1. Copia `faro.env.example` como `faro.env`.
2. Completa la cuenta SMTP que **envía** los correos (con Gmail, usa una
   *contraseña de aplicación*).

Sin `faro.env` la app funciona igual: los reportes se guardan y quedan
"pendientes de notificar"; se envían automáticamente la próxima vez que se
abra la app con el correo configurado (también hay un botón "Reintentar envío").

## Pruebas

```bash
cargo test
```

## Arquitectura (Clean Architecture + SOLID)

```
src/
├── domain/          Entidades y reglas de negocio + contratos (traits). Sin dependencias.
│   ├── report.rs        Report, NewReport, Category, Severity, Status, validación
│   ├── repository.rs    ReportRepository, NotificationOutbox
│   ├── notifier.rs      Notifier
│   └── clock.rs         Clock
├── application/     Casos de uso: depende solo del dominio.
│   ├── report_service.rs        Registrar, consultar, cambiar estado, reintentar
│   ├── dispatcher.rs            Envío de correos en segundo plano (no bloquea la UI)
│   ├── notification_message.rs  Contenido del correo
│   └── summary.rs               Estadísticas
├── infrastructure/  Adaptadores concretos.
│   ├── sqlite_repository.rs     SQLite (rusqlite)
│   ├── smtp_notifier.rs         SMTP (lettre + rustls)
│   ├── disabled_notifier.rs     Objeto nulo cuando no hay correo configurado
│   └── config.rs                Lectura de faro.env / variables FARO_*
├── ui/              Presentación (egui).
│   ├── glass.rs     Render de vidrio con brillo especular por vértice
│   ├── light.rs     Puntero-luz
│   ├── motion.rs    Motor de resortes
│   ├── widgets.rs   Botones, campos, selector segmentado, interruptor…
│   └── pages/       Nuevo reporte · Reportes · Resumen
└── main.rs          Raíz de composición: conecta implementaciones con contratos.
```

| Principio | Dónde se aplica |
|---|---|
| **S** responsabilidad única | Cada módulo hace una cosa: persistir, notificar, formatear, dibujar. |
| **O** abierto/cerrado | Un canal nuevo (Telegram, SMS) se agrega implementando `Notifier` sin tocar el resto. |
| **L** sustitución de Liskov | `SmtpNotifier`, `DisabledNotifier` y el notificador de pruebas son intercambiables. |
| **I** segregación de interfaces | `ReportRepository` y `NotificationOutbox` están separados. |
| **D** inversión de dependencias | `ReportService` recibe `Arc<dyn …>`; solo `main.rs` conoce SQLite y SMTP. |

Patrones usados: Repository, Outbox (ningún reporte se pierde si falla el
correo), Null Object, Builder (`TextField`, `Glass`), Composition Root.

## Rendimiento en equipos modestos

- Backend OpenGL liviano (glow); funciona en Windows, macOS y Linux (X11/Wayland).
- La ventana solo se redibuja cuando hay entrada o una animación en curso: en
  reposo el uso de CPU es prácticamente cero.
- El correo se envía en un hilo aparte; la interfaz nunca se congela.
