# Medición del editor nativo

Resultado del 30 de septiembre de 2026, compilación Release, Qt 6.11.2 y Rust
1.98.1, Linux x86_64. Equipo: Intel Core i3-1115G4, 2 núcleos físicos y 4 hilos,
7.5 GiB de RAM. No es la máquina de referencia de cuatro núcleos físicos.

Cada escena usa una cuadrícula, personajes con PNG distintos de 128 × 128,
Discord desconectado y comprobaciones de actualización desactivadas. El editor
se abre con el backend de renderizado por software de Qt, plataforma offscreen,
y escala 1. Se esperan tres segundos y se muestrea el proceso durante treinta
segundos. RSS incluye páginas compartidas; PSS las distribuye entre procesos.
No se incluyen navegador, OBS, Streamlabs ni memoria de GPU.

| Invitados | RSS medio | RSS máximo muestreado | PSS medio | CPU, porcentaje de un núcleo |
| --- | ---: | ---: | ---: | ---: |
| 8 | 131.74 MiB | 131.82 MiB | 113.26 MiB | 0.033% |
| 32 | 144.59 MiB | 144.67 MiB | 125.99 MiB | 0.000% |
| 128 | 201.47 MiB | 201.55 MiB | 182.85 MiB | 0.000% |

Cero significa que no avanzó un tick de CPU durante el intervalo; no demuestra
coste nulo. La resolución del muestreo es aproximadamente 0.033% de un núcleo.
Son medidas de reposo con arte estático, no límites de RAM ni un benchmark de
animaciones. GIF, WebP, WebM, resplandor, composición por GPU y muchas fuentes
abiertas pueden cambiar el consumo. La importación de WebM usa procesos de
FFmpeg temporales; no quedan decodificadores de vídeo en segundo plano en reposo.

Reproducir desde el directorio del proyecto:

```sh
cmake -S . -B build-release -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF
cmake --build build-release --parallel 2
python packaging/measure-linux.py --seconds 30 --output dist/measurements.json
```

`--platform wayland` y `--platform xcb` permiten medir una ventana real con los
controladores del escritorio. Esos escenarios, escenas animadas y Windows
siguen pendientes. El script crea y elimina sus propios datos de prueba.
