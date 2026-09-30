# Third-party components

Charlita's original source is MIT licensed. Native distributions include
unmodified, dynamically linked Qt 6.8.3 libraries and separately executed
LGPL FFmpeg/ffprobe tools. You can replace the shared Qt libraries. Charlita
places no restriction on modifying these components or debugging those changes.

- Qt source and build instructions: https://download.qt.io/archive/qt/6.8/6.8.3/
- Qt modules, copyright attributions and license texts: `licenses/Qt/`.
- Qt component inventory: https://doc.qt.io/qt-6.8/licenses-used-in-qt.html
- Rust dependency copyright and license texts: `licenses/Rust.json`.
- Inter Variable 4.1, by Rasmus Andersson: `licenses/Inter-OFL.txt`;
  https://github.com/rsms/inter/tree/v4.1.
- FFmpeg build provenance and supplied notices: `licenses/FFmpeg/`.
- FFmpeg source revision and dependency build recipes are recorded in
  `licenses/FFmpeg/FFmpeg-build.txt`; the complete upstream build system is
  https://github.com/BtbN/FFmpeg-Builds.

The release build copies license and attribution files from the matching Qt
source archives, rather than relying on the application license to cover them.
Do not publish distribution artifacts with missing or unresolved notices.
