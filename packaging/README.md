# Distribuciones nativas

Actions compila con Qt 6.8.3 oficial y Rust estable en Ubuntu 22.04 y Windows
2022/MSVC. El paquete Linux requiere glibc 2.35 o posterior. Las compilaciones
hechas con el Qt de otra distribución pueden exigir una glibc más reciente;
no se deben etiquetar como equivalentes a las de Actions.

Los paquetes incluyen Qt compartido, el ejecutable y FFmpeg/ffprobe LGPL
verificados mediante SHA-256. El script de FFmpeg fija una compilación mensual
retenida dos años por su proveedor; revisa y actualiza ese pin antes de que
expire. No se descarga FFmpeg cuando el usuario abre la aplicación.

```sh
python packaging/fetch-tools.py linux
cargo install cargo-bundle-licenses --version 4.2.0 --locked
cmake --install build --prefix "$PWD/dist/stage"
python packaging/package.py --stage dist/stage --qt-sources /ruta/Qt/6.8.3/Src
```

Windows utiliza `windows` en el primer comando y requiere NSIS en el PATH o en
su ubicación estándar. Qt sources debe corresponder a la versión de las
bibliotecas desplegadas; el paquete copia sus avisos de atribución y licencias.
En Linux, instala también `patchelf` antes de desplegar: el paso de instalación
ajusta las bibliotecas incluidas para que sus dependencias se encuentren en
la misma carpeta, incluso cuando difieren de las bibliotecas del escritorio.

Resultados:

- `charlita-VERSION-windows-x86_64-setup.exe`: instalación por usuario y desinstalador.
- `charlita-VERSION-windows-x86_64-portable.zip`: abre `Charlita.cmd` o `bin/charlita.exe`.
- `charlita-VERSION-linux-x86_64.AppImage`: ejecutable autónomo; acepta `--portable`.
- `charlita-VERSION-linux-x86_64.deb`: instalación con el gestor de paquetes.
- `charlita-VERSION-linux-x86_64-portable.tar.gz`: extrae y abre `Charlita/charlita`.
- `SHA256SUMS`: hashes de los artefactos.

Los builds de ramas suben artefactos de Actions y no publican una Release.
Para una Release estable, completa primero [la verificación](../docs/verification.md),
crea una etiqueta que coincida con Cargo.toml, reúne los paquetes de ambos
sistemas y genera un único `SHA256SUMS` con todos ellos. Charlita comprueba
exclusivamente Releases estables publicadas; ignora borradores y prereleases.

Desinstalar conserva los datos del usuario. Para actualizar un portable,
extrae en otra carpeta y copia `bin/data/` con Charlita cerrada; en el AppImage,
`data/` está junto al archivo AppImage. Nunca sobrescribas
el ejecutable que mantiene una fuente de stream abierta.

La comprobación de arranque elimina del entorno las rutas del SDK de Qt para
evitar que una biblioteca ausente pase inadvertida. En el runner de Windows,
`check-windows-installer.py` también instala el `.exe` en una carpeta temporal,
abre y captura la ventana instalada y verifica que desinstalar elimina el
ejecutable y conserva `bin/data/`. Esa prueba no sustituye una revisión de la
bandeja, el compositor ni los diálogos en un escritorio Windows real.
