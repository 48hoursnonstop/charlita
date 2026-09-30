# Verificación de Charlita

Una compilación correcta no demuestra por sí sola compatibilidad con Discord,
OBS, Streamlabs, un gestor de ventanas o una instalación limpia de Windows.
Este registro separa resultados ejecutados de comprobaciones pendientes.

## Comprobaciones automatizadas

- `cargo test --locked --all-targets`: 14 pruebas del motor. Geometría sin límite
  de participantes, lienzo automático, coordenadas libres negativas, migración
  de proyectos con tamaño fijo, reflujo y espacios conservados, draft/Apply atómico, bloqueo de instancia,
  persistencia de URL, paquetes con checksums, frames RPC y desconexión.
- La prueba de servidor usa solicitudes HTTP y WebSocket reales: el borrador
  no se publica, las simulaciones no salen al stream, Apply envía el cambio,
  show/hide es inmediato, hay rangos HTTP para WebM y fotogramas estáticos.
- `ctest --test-dir build --output-on-failure`: la aplicación QML de producción,
  con su motor real, editada mediante eventos de teclado y ratón. Verifica
  seis casos de comportamiento: biblioteca/importación, paneles, Apply, ventana
  de 320 px, aviso de error sobre un panel, transparencia del WebM en la vista
  previa nativa, movimiento reducido y cambio entre lienzo automático y fijo.
- `QT_SCALE_FACTOR=2 ctest --test-dir build --output-on-failure`: los mismos
  casos con escala de pantalla del 200% en Linux.
- `cmake --build build --target charlita_qmllint`: comprobación de tipos QML.
- `cargo clippy --locked --all-targets -- -D warnings` y `cargo fmt --all --check`.
- Actions repite esas comprobaciones en Linux y Windows, despliega Qt y ejecuta
  `packaging/check-startup.py` para abrir y capturar su propia ventana. El script
  elimina las rutas del SDK de Qt del entorno para verificar las bibliotecas
  incluidas. El empaquetador rechaza una distribución sin Qt Core o el módulo
  Qt Quick Controls. En Windows, el arranque desplegado utiliza el backend
  nativo `windows`; las pruebas QtTest utilizan `offscreen` del SDK.

Las capturas de QtTest se obtienen del propio `QQuickWindow`, usando
`CHARLITA_UI_CAPTURE_DIR`; no capturan el escritorio del usuario. El ejecutable
admite `--screenshot ARCHIVO` para revisar una composición local de prueba.
La revisión del flujo de importación y publicación está en
[interface-review.md](interface-review.md). Estas pruebas usan datos aislados.
En el navegador de prueba, la salida de ocho invitados con lienzo automático
presenta ocho PNG cargados, tamaño natural de 1072 × 628, escalado al viewport
y fondo `rgba(0, 0, 0, 0)`. Eso verifica la salida web, no su integración con
los clientes OBS y Streamlabs.

## Distribuciones comprobadas

El [run 36707451579 de Actions](https://github.com/48hoursnonstop/charlita/actions/runs/36707451579),
del 30 de septiembre de 2026, terminó correctamente en Linux y Windows para
el código `2b7c5837b81fc076fcdaeed304919a882f640ca1`, con Qt 6.8.3.
Incluye las pruebas del motor, QtTest, lint, empaquetado y arranque desplegado.

- Windows: el paquete desplegado abrió y capturó su ventana con el backend
  nativo. El instalador NSIS se instaló en una carpeta temporal del runner;
  la aplicación instalada también abrió y capturó su ventana. La desinstalación
  eliminó el ejecutable y conservó un archivo de prueba en `bin/data/`.
- Linux: el portable de ese run se descargó y contrastó con su entrada en
  `SHA256SUMS`. El ejecutable extraído abrió y capturó su ventana tanto con
  `offscreen` como con Wayland en esta máquina Arch Linux, sin rutas del SDK en
  el entorno. El registro del cargador confirmó Qt Core incluido en el paquete
  y las bibliotecas del escritorio para C++, fuentes y Wayland. La captura
  procede de la propia ventana de Charlita.

Estas comprobaciones no prueban un gestor de paquetes `.deb`, FUSE/AppImage,
todos los escritorios ni una instalación interactiva en otra máquina. Los
paquetes están en los artefactos de Actions; aún no hay una Release estable.

## Antes de publicar una Release estable

| Comprobación | Resultado requerido | Estado |
| --- | --- | --- |
| Aplicación Discord registrada | ID público y acceso RPC de voz concedido | Pendiente externo |
| OAuth cliente público + PKCE con scopes RPC | Autorización de cuenta externa aceptada por Discord | No verificado |
| Discord servidor, DM y grupo | Estado de voz, seguimiento, mute y reconexión correctos | No verificado con llamadas reales |
| OBS Studio Linux y Windows | Browser Source transparente y cambios en directo | Pendiente de cliente OBS |
| Streamlabs Desktop Windows | Browser Source transparente y WebM | No verificado |
| Ventana y bandeja Windows | Abrir, cerrar a bandeja, segunda instancia y salir | Arranque nativo desplegado e instalado probado en CI; bandeja y segunda instancia pendientes de escritorio Windows |
| Hotkeys X11, Wayland y Windows | Activación global y conflicto comunicado | Pendiente de escritorios reales |
| GIF, WebP animado y WebM alpha | Animación y transparencia, reposo con movimiento reducido | Vista previa WebM alpha y movimiento reducido probados con QtTest en ambos sistemas; resto de formatos y clientes de stream pendientes |
| Instalación limpia y desinstalación | Qt y FFmpeg incluidos, datos conservados | Instalación/desinstalación Windows y arranque portable Linux probados; `.deb`, AppImage y escritorios limpios pendientes |
| Lectores de pantalla | Nombres, foco, paneles y cambios de estado comprensibles | No verificado |
| Actualización con Release publicada | Checksum, variante correcta y acción manual | Pendiente de Release |
| Consumo con escenas de 8, 32 y 128 invitados | Medidas reproducibles, sin prometer RAM constante | Medido en reposo en Linux; [método y límites](performance.md) |

La autorización documentada por Discord para Social SDK no constituye por sí
sola una confirmación del flujo RPC de Charlita. Ninguna simulación ni prueba
con un servidor ficticio puede sustituir esa validación.
