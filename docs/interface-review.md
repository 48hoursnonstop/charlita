# Revisión del flujo de importación y publicación

## Alcance y cobertura

Revisión de un flujo completo: escena vacía → importar arte → biblioteca →
editar personaje → asignarlo a un invitado → aplicar → composición publicada.
Incluye importación en curso, error con el editor abierto, lienzo automático
y fijo, ventana de 320 × 640 y escala de pantalla del 200%. Fecha: 30 de
septiembre de 2026.

La interfaz usa Qt Quick/QML, controles Basic, componentes compartidos
`ActionButton`, `Field`, `Check` y `FocusScroll`, tokens de `Theme.qml` y la
fuente Inter Variable incluida. Las convenciones del producto están en
`README.md` y `docs/product.md`. No hay una guía independiente de diseño.
Esta revisión no cubre el consentimiento externo de Discord, instaladores,
actualizaciones, todos los controles de perfiles ni llamadas reales.

| Dominio | Evidencia inspeccionada | Resultado |
| --- | --- | --- |
| Accesibilidad | Nombres accesibles y foco de controles; teclado en escena y editores; Tab en 35 controles del editor estrecho; Escape sobre aviso y panel; reducción de movimiento | Clear en el flujo revisado |
| Layout | Escena vacía y aplicada, biblioteca, panel de personaje, recorte a una columna, barras de desplazamiento visibles, foco que se desplaza al entrar en controles | Clear en el flujo revisado |
| Escritura | Acciones de importar, asignar, aplicar y copiar URL; borrador frente a publicación; mensajes vacío, carga y error; textos ES/EN en QML | Clear en el flujo revisado |
| Tipografía | Inter Variable cargada desde recursos, texto envuelto en panel estrecho, nombres completos accesibles desde el editor y tooltip, etiquetas dentro del lienzo | Clear en el flujo revisado |
| Color | Tokens de texto, texto secundario, botón principal, peligro y foco; medidas de contraste; texto de estado junto al indicador de conexión | Clear en el flujo revisado |
| UI | Jerarquía de superficies y acciones, foco del botón principal, paneles sin docks, aviso visible sobre el editor, salto y animación sujetos a movimiento reducido | Clear en el flujo revisado |

## Hallazgos

No actionable interface findings en el alcance inspeccionado después de las
correcciones. Los avisos se cierran antes que el editor con Escape. El recorte
se dispone en una columna en ventanas estrechas y los paneles muestran cuándo
hay más contenido. El tamaño automático reserva espacio para nombres y saltos.

## Verificación

Comprobaciones ejecutadas en Linux con Qt 6.11.2:

- `cargo test --locked --all-targets`: 14 pruebas del motor.
- `cmake --build build --target charlita_qmllint`: sin advertencias.
- `CHARLITA_UI_CAPTURE_DIR=/tmp/charlita-ui-final ctest --test-dir build --output-on-failure`:
  seis casos de comportamiento de la QML de producción con el motor real.
- `QT_SCALE_FACTOR=2 CHARLITA_UI_CAPTURE_DIR=/tmp/charlita-ui-scale2 ctest --test-dir build --output-on-failure`:
  los mismos casos con escala del 200%.
- Capturas obtenidas mediante `QQuickWindow::grabWindow`: vacío, biblioteca,
  personaje, recorte estrecho, foco con teclado, aviso de error, importación,
  WebM transparente, composición aplicada, expandida y automática.
- El WebM de prueba se importa realmente. `QImageReader` comprueba alfa cero
  en una esquina y rojo en el centro de su vista previa animada; al activar
  movimiento reducido, `AnimatedImage.playing` pasa a falso.
- Contrastes calculados con luminancia relativa sRGB: texto/superficie
  14.57:1; texto secundario/superficie 7.78:1; texto secundario/superficie elevada
  6.82:1; texto y borde de foco del botón principal/acento 10.40:1;
  peligro/superficie elevada 7.51:1; foco/fondo 11.59:1.

**Not verified:** interacción con lectores de pantalla, aumento de tamaño de
fuente independiente de la escala de pantalla, escritorio nativo de Windows,
todos los formatos animados y el comportamiento de fuentes dentro de OBS o
Streamlabs. Escala de pantalla del 200% no equivale a verificar esos escenarios.

## Veredicto

**Approve** para el flujo y los estados inspeccionados: no quedan hallazgos
HIGH. Este resultado no sustituye las comprobaciones externas de
[verification.md](verification.md).
