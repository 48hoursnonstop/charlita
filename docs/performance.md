# Medición del editor nativo

Medición inicial del commit `49cd39b`, el 30 de septiembre de 2026,
compilación Release, Qt 6.11.2 y Rust
1.98.1, Linux x86_64. Equipo: Intel Core i3-1115G4, 2 núcleos físicos y 4 hilos,
7.5 GiB de RAM. No es la máquina de referencia de cuatro núcleos físicos.

Cada escena usa una cuadrícula con lienzo fijo de 1280 × 720,
personajes con PNG distintos de 128 × 128,
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

Una segunda medición del mismo día usa el lienzo automático de `32c05a7`,
con las correcciones del aviso de Escape y de las etiquetas en el editor,
y el mismo método y equipo:

| Invitados | RSS medio | RSS máximo muestreado | PSS medio | CPU, porcentaje de un núcleo |
| --- | ---: | ---: | ---: | ---: |
| 8 | 132.00 MiB | 132.11 MiB | 108.92 MiB | 0.000% |
| 32 | 152.04 MiB | 152.12 MiB | 131.59 MiB | 0.000% |
| 128 | 230.98 MiB | 231.06 MiB | 211.33 MiB | 0.000% |

El lienzo automático conserva el tamaño solicitado de los personajes. El fijo
reduce los personajes para acomodarlos; esas geometrías distintas también
cambian el tamaño al que Qt solicita las imágenes. El resultado no permite
atribuir toda la diferencia a una sola causa.

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
python packaging/measure-linux.py --canvas fixed --seconds 30 --output dist/measurements.json
python packaging/measure-linux.py --seconds 30 --output dist/measurements-auto.json
```

El script usa el lienzo automático por defecto; `--canvas fixed` reproduce
el escenario de la tabla. `--platform wayland` y `--platform xcb` permiten medir una ventana real con los
controladores del escritorio. Esos escenarios, escenas animadas y Windows
siguen pendientes. El script crea y elimina sus propios datos de prueba.
