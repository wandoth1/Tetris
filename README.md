# Tetris · Neon Pulse

**Encuentra tu ritmo. Encaja el caos.**

Juego de bloques en Rust para Windows y macOS, con estética neón, renderizado GPU y tres temas originales de música sintetizada. Sin cuentas, anuncios, conexión a Internet ni archivos de audio que tengas que instalar.

[![Build](https://github.com/wandoth1/Tetris/actions/workflows/build.yml/badge.svg)](https://github.com/wandoth1/Tetris/actions/workflows/build.yml)

## Descargar y jugar

Los paquetes se publican en [Releases](https://github.com/wandoth1/Tetris/releases). La compilación automática de `main` aparece como **Neon Pulse — development build** cuando todas las pruebas y paquetes terminan correctamente.

| Equipo | Archivo |
| --- | --- |
| Windows 10/11, 64 bits | `Tetris-NeonPulse-Windows-x64.zip` |
| Mac con Apple Silicon | `Tetris-NeonPulse-macOS-AppleSilicon.zip` |
| Mac con Intel | `Tetris-NeonPulse-macOS-Intel.zip` |
| Música, independiente del juego | `NeonPulse-Soundtrack.zip` |

En Windows, extrae el ZIP y abre `Tetris.exe`. En macOS, extrae el ZIP y mueve `Tetris Neon Pulse.app` a Aplicaciones. Los ZIP contienen el juego completo; **no necesitas Rust para jugar**.

Los ejecutables no tienen certificado comercial de Windows ni notarización de Apple. La aplicación de Mac tiene firma *ad hoc*, que no equivale a una firma Developer ID. El sistema puede mostrar un aviso de desarrollador no identificado. Comprueba que el archivo procede de este repositorio; no desactives globalmente la seguridad del sistema. También puedes compilar desde el código fuente. Referencia de Apple: https://support.apple.com/es-es/102445

## Tres modos

**Maratón.** La velocidad aumenta cada diez líneas. Consigue combos y supera tu puntuación.

**40 líneas.** Completa cuarenta líneas en el menor tiempo posible. El mejor tiempo se guarda por separado.

**Zen.** Sin gravedad automática. Tú decides cuándo baja cada pieza. Una vez apoyada, conserva la fijación normal para poder practicar los giros.

## Mecánicas

Tablero visible de 10 × 20, siete piezas repartidas en bolsas equilibradas de siete, rotación SRS con desplazamientos junto a paredes y suelo, pieza fantasma, reserva y cinco próximas piezas. Incluye caída suave y directa, fijación de 500 ms con un máximo de quince reinicios, combos, bonificación consecutiva de jugadas difíciles, T-spins y perfect clears.

La puntuación de una, dos, tres o cuatro líneas es 100 / 300 / 500 / 800 multiplicado por el nivel. La caída suave suma un punto por casilla y la directa, dos. Los T-spins y combos tienen sus propias bonificaciones; las reglas exactas están en `src/engine.rs`. Es una implementación independiente, no una certificación del reglamento oficial.

## Controles

| Acción | Tecla |
| --- | --- |
| Mover | Izquierda / derecha o A / D |
| Bajar | Abajo o S |
| Girar a la derecha | Arriba, W o X |
| Girar a la izquierda | Z |
| Caída directa | Espacio |
| Reservar / intercambiar | C o Shift |
| Pausa / continuar | P o Esc; Enter continúa |
| Reiniciar | Desde pausa, R y otra vez R para confirmar |
| Menú | Q desde pausa; Esc o Q al terminar |
| Música | M |
| Siguiente tema | N |
| Efectos sonoros | V |
| Volumen | + / -; también teclado numérico |
| Movimiento reducido | F2 |
| Pantalla completa | F11 |

El menú admite ratón, flechas y 1/2/3. Enter inicia una partida. El juego conserva las proporciones al redimensionar la ventana, incluido en pantallas de alta densidad. F2 elimina partículas, sacudidas y movimiento ambiental, y reduce el destello de líneas.

## Música hecha en Rust

Tres composiciones originales: **Neon Descent** (120 BPM), **Night Circuit** (132 BPM) y **Afterglow** (96 BPM). Bajo, arpegios, melodía, pad y batería se generan mediante osciladores, envolventes, ruido determinista y eco estéreo. No se han utilizado grabaciones comerciales ni la melodía de una edición oficial.

`build.rs` sintetiza PCM WAV estéreo y lo incrusta en el ejecutable. No requiere MIDI, un sintetizador instalado, soundfonts ni ficheros externos. El indicador visual pulsa al tempo del tema; no es un analizador del audio de salida.

Para exportar las tres pistas:

```sh
cargo run --release --no-default-features --bin soundtrack -- soundtrack
```

## Compilar

Instala Rust estable con Cargo. En Windows usa la cadena MSVC y las herramientas de compilación de Visual Studio. En macOS instala las herramientas de Xcode (`xcode-select --install`).

```sh
git clone https://github.com/wandoth1/Tetris.git
cd Tetris
cargo run --release --bin tetris
```

Para obtener el mismo conjunto de dependencias de una compilación publicada, utiliza el `Cargo.lock` incluido en sus metadatos y añade `--locked`. El flujo resuelve las dependencias una vez y comparte ese fichero entre las tres plataformas.

```sh
cargo test --no-default-features --lib
cargo clippy --no-default-features --all-targets
cargo fmt --all
```

Sin audio, por ejemplo para diagnosticar un dispositivo de sonido:

```sh
cargo run --release --no-default-features --features desktop --bin tetris
```

En Linux (usado para las pruebas de renderizado) hacen falta las bibliotecas de desarrollo X11/OpenGL y ALSA para la compilación con sonido. El flujo `build.yml` documenta los paquetes utilizados.

## Arquitectura y pruebas

- `src/engine.rs`: lógica determinista sin dependencias de gráficos ni audio.
- `src/storage.rs`: récords y preferencias locales, con validación y escritura mediante fichero temporal.
- `src/synth.rs`: compositor/sintetizador y codificador WAV.
- `src/main.rs`, `src/visual.rs`, `src/audio.rs`: controles, interfaz y reproducción.
- `scripts/package.py`: empaquetado de Windows y macOS, icono generado y firma ad hoc de la aplicación de Mac.

La batería de pruebas cubre bolsas de piezas, colisiones, giros, caídas, reserva, fijación, limpieza de filas, puntuación, final de sprint, persistencia y formato de audio. Un ensayo con entradas aleatorias verifica invariantes del tablero. CI ejecuta pruebas nativas en Windows, Mac Intel y Mac Apple Silicon, y abre una ventana con Xvfb en Linux para capturar un fotograma real del renderizador. Una compilación o prueba automatizada correcta no sustituye a probar los controles y el sonido en tu equipo.

El modo de comprobación no toca tus récords:

```sh
cargo run --release --no-default-features --features desktop --bin tetris -- --smoke-test --screenshot screenshot.png
```

## Datos locales

Sólo se guardan récords y preferencias; no se recopila telemetría.

- Windows: `%LOCALAPPDATA%/TetrisNeonPulse/records-v1.ini`
- macOS: `~/Library/Application Support/TetrisNeonPulse/records-v1.ini`
- Linux: `$XDG_DATA_HOME/TetrisNeonPulse/records-v1.ini`, o `~/.local/share/TetrisNeonPulse/records-v1.ini`

La partida en curso no se reanuda después de cerrar el programa. Los récords se guardan periódicamente y al pausar o salir. Una ubicación de guardado no disponible se indica en pantalla y no bloquea el juego.

## Publicaciones

Cada cambio en `main` ejecuta las comprobaciones y actualiza una **prepublicación de desarrollo** únicamente si todo termina bien. Las etiquetas `v*` generan publicaciones versionadas. Los ZIP tienen sumas SHA-256. No se necesitan tokens personales ni certificados para este flujo; utiliza el token limitado de GitHub Actions.

Código, música e icono originales bajo licencia MIT. Proyecto independiente, sin afiliación ni respaldo de los responsables de Tetris. La licencia del código no concede derechos sobre marcas de terceros.
