# Charlita

Personajes de tus invitados de Discord para OBS Studio y Streamlabs Desktop.
Editor nativo en **Qt 6 Quick/QML**, motor en **Rust** y salidas transparentes
locales. Windows y Linux; software libre bajo MIT.

## Estado de verificación

El motor y la interfaz se prueban con HTTP/WebSocket y QtTest. **Todavía no se
ha validado una llamada real de Discord**: falta registrar la aplicación y
obtener acceso a RPC de voz. La autorización de cliente público con PKCE debe
validarse con esa aplicación; no se presenta como compatibilidad confirmada.
Los paquetes de distribución se generan en Actions y requieren la verificación
manual descrita en [docs/verification.md](docs/verification.md) antes de publicar
una versión estable.

## Uso

1. Abre Charlita. En **Biblioteca**, importa el arte de un personaje: PNG, JPEG,
   WebP, GIF, WebM VP8/VP9 o una hoja de sprites.
2. En **Escena**, añade invitados por su ID de Discord, o selecciona los que
   aparezcan cuando Discord esté conectado. Asigna su personaje habitual o
   uno distinto para el grupo.
3. Elige fila, columna, cuadrícula o posiciones libres en **Composición**.
   Arrastra personajes en el modo libre; también puedes moverlos con flechas,
   usar Shift para pasos de diez píxeles, o escribir las coordenadas.
4. Pulsa **Aplicar cambios**. La edición se guarda automáticamente como borrador;
   la fuente del stream conserva la última composición aplicada.
5. Copia la URL del grupo y añádela como **Fuente de navegador** en OBS o Streamlabs.
   Usa las dimensiones indicadas en la composición. El fondo es transparente.

Mostrar, ocultar y cambiar expresiones desde los controles en directo actúa
inmediatamente. Las pruebas de hablar/silencio del editor solo afectan a la
vista previa. Al perder Discord, los personajes quedan en reposo y se mantienen
los controles manuales. Cada grupo tiene una URL persistente; la salida
individual se encuentra en el panel del invitado.

Para compartir arte, exporta un personaje `.charlita` desde su editor. Un
proyecto exportado incluye perfiles, composiciones y archivos; no incluye
credenciales ni la configuración de conexión de la máquina.

Cerrar la ventana deja el motor en la bandeja cuando el escritorio ofrece una.
**Salir** detiene Charlita y sus fuentes locales. Sin bandeja, cerrar la ventana
termina la aplicación.

## Discord

Consulta [docs/discord.md](docs/discord.md). Charlita usa el cliente oficial de
escritorio y su IPC; no acepta tokens de cuenta, no usa self-bots y no necesita
que los invitados instalen nada. La configuración admite seguir la llamada
actual o fijar un canal. Los invitados descubiertos deben añadirse explícitamente
antes de aparecer en una fuente.

## Compilar

Necesitas Rust **1.95 o posterior**, CMake 3.24+, Ninja, un compilador C++20 y
Qt **6.8 o posterior** con Core, Gui, Qml, Quick, QuickControls2, Widgets,
Svg y Test. Linux también necesita los paquetes de desarrollo
X11, Wayland y D-Bus. FFmpeg/ffprobe se usan para importar WebM.
La vista previa nativa usa una copia animada de hasta 512 px y 30 fps;
OBS y Streamlabs reciben el archivo original con su transparencia.

```sh
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel 2
./build/charlita
```

Windows: ejecuta los comandos desde una consola de desarrollo MSVC 2022 con
Qt en `CMAKE_PREFIX_PATH`, y abre `build/charlita.exe`.

```sh
cargo test --locked --all-targets
cmake --build build --target charlita_qmllint
ctest --test-dir build --output-on-failure
```

Los archivos QML se compilan y se incluyen en el ejecutable. Qt se enlaza como
bibliotecas compartidas. La interfaz llama al motor por una pequeña ABI de C;
no hay navegador ni servidor HTTP de edición dentro de la aplicación.
El servidor de salida escucha exclusivamente en `127.0.0.1`.

## Datos, portable y actualizaciones

Los datos normales se guardan en la carpeta de aplicación del usuario.
`--portable` guarda `data/` junto al ejecutable o al AppImage. Las distribuciones
portable contienen `portable.flag` junto al ejecutable para activar este modo.
`--data-dir RUTA` permite una carpeta independiente y `--port N` cambia el
puerto local. Los perfiles y borradores se recuperan desde SQLite al reiniciar.

Las comprobaciones de GitHub Releases son automáticas si están activadas.
Descargar una actualización requiere una acción explícita, verifica
`SHA256SUMS` y deja el archivo listo para abrir. No cierra la aplicación ni
sustituye una instalación durante el stream. Las distribuciones portable deben
extraerse en una carpeta nueva y conservar su `data/`.

Consulta [packaging/README.md](packaging/README.md) para generar paquetes.
La lista acordada del producto está en [docs/product.md](docs/product.md).
