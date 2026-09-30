# Verificación de Charlita

Una compilación correcta no demuestra por sí sola compatibilidad con Discord,
OBS, Streamlabs, un gestor de ventanas o una instalación limpia de Windows.
Este registro separa resultados ejecutados de comprobaciones pendientes.

## Comprobaciones automatizadas

- `cargo test --locked --all-targets`: geometría sin límite de participantes,
  reflujo y espacios conservados, draft/Apply atómico, bloqueo de instancia,
  persistencia de URL, paquetes con checksums, frames RPC y desconexión.
- La prueba de servidor usa solicitudes HTTP y WebSocket reales: el borrador
  no se publica, las simulaciones no salen al stream, Apply envía el cambio,
  show/hide es inmediato, hay rangos HTTP para WebM y fotogramas estáticos.
- `ctest --test-dir build --output-on-failure`: la aplicación QML de producción,
  con su motor real, editada mediante eventos de teclado y ratón. Verifica
  biblioteca/importación, paneles, Apply y ventana estrecha.
- `cmake --build build --target charlita_qmllint`: comprobación de tipos QML.
- `cargo clippy --locked --all-targets -- -D warnings` y `cargo fmt --all --check`.
- Actions repite esas comprobaciones en Linux y Windows, despliega Qt y prueba
  el arranque del binario desplegado en Linux.

Las capturas de QtTest se obtienen del propio `QQuickWindow`, usando
`CHARLITA_UI_CAPTURE_DIR`; no capturan el escritorio del usuario. El ejecutable
admite `--screenshot ARCHIVO` para revisar una composición local de prueba.

## Antes de publicar una Release estable

| Comprobación | Resultado requerido | Estado |
| --- | --- | --- |
| Aplicación Discord registrada | ID público y acceso RPC de voz concedido | Pendiente externo |
| OAuth cliente público + PKCE con scopes RPC | Autorización de cuenta externa aceptada por Discord | No verificado |
| Discord servidor, DM y grupo | Estado de voz, seguimiento, mute y reconexión correctos | No verificado con llamadas reales |
| OBS Studio Linux y Windows | Browser Source transparente y cambios en directo | Pendiente de cliente OBS |
| Streamlabs Desktop Windows | Browser Source transparente y WebM | No verificado |
| Ventana y bandeja Windows | Abrir, cerrar a bandeja, segunda instancia y salir | Pendiente de escritorio Windows |
| Hotkeys X11, Wayland y Windows | Activación global y conflicto comunicado | Pendiente de escritorios reales |
| GIF, WebP animado y WebM alpha | Animación y transparencia, reposo con movimiento reducido | Pendiente de todos los formatos en ambos sistemas |
| Instalación limpia y desinstalación | Qt y FFmpeg incluidos, datos conservados | Pendiente de los paquetes compilados |
| Lectores de pantalla | Nombres, foco, paneles y cambios de estado comprensibles | No verificado |
| Actualización con Release publicada | Checksum, variante correcta y acción manual | Pendiente de Release |
| Consumo con escenas de 8, 32 y 128 invitados | Medidas reproducibles, sin prometer RAM constante | Pendiente de benchmark optimizado |

La autorización documentada por Discord para Social SDK no constituye por sí
sola una confirmación del flujo RPC de Charlita. Ninguna simulación ni prueba
con un servidor ficticio puede sustituir esa validación.
