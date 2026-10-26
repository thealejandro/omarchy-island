# omarchy-island

[English](README.md)

Una isla dinámica para [Omarchy](https://omarchy.org) que sale del centro de
la barra. Muestra lo que suena, te da controles de música, el calendario del
mes y un temporizador de enfoque, y se aparta cuando terminas.

Es un plugin normal del shell de Omarchy: corre dentro de `omarchy-shell`,
toma los colores, la fuente y el acento de tu tema y no reemplaza ni modifica
nada de Omarchy. Si lo quitas, tu escritorio queda exactamente como estaba.

## Funciones

- **Vistazo**: cuando cambia la canción o termina el temporizador, una línea
  baja de la barra unos segundos.
- **Abrir**: pasa el ratón por el borde superior sobre el reloj, o justo
  debajo del centro de la barra (o usa un atajo), y se abre la isla: portada,
  reproductor, título, artista, barra de progreso que puedes pulsar o
  arrastrar, anterior / reproducir / siguiente.
- **Calendario**: el mes actual con hoy resaltado. Las flechas o la rueda del
  ratón cambian de mes; pulsa el título para volver a hoy.
- **Temporizador de enfoque**: Pomodoro (25 / 5 por defecto) con una
  notificación al terminar cada fase. Usa la rueda sobre el temporizador
  para cambiar la duración del enfoque (5–120 min).
- **Español e inglés**: sigue `LANG`, o elígelo en `shell.json`.
- **Sigue tu tema**: cambia en vivo con `omarchy theme set` (colores, fuente
  y acento). La isla tiene esquinas redondeadas con "orejas" cóncavas que se
  funden con la barra.
- **Ligera**: sin procesos extra ni sondeos en reposo. La música llega por
  MPRIS (Spotify, navegadores, mpv…).

## Requisitos

Omarchy 4 o posterior (`omarchy-shell` basado en Quickshell) con la barra
arriba.

## Instalación

```bash
omarchy plugin add https://github.com/thealejandro/omarchy-island.git --enable
```

Para quitarlo:

```bash
omarchy plugin remove thealejandro.island
```

## Uso

- Pasa el ratón por el borde superior de la pantalla sobre el reloj, o por la
  franja fina justo debajo del centro de la barra; al alejarte se cierra.
- Ábrela con un atajo, en `~/.config/hypr/bindings.lua`:

  ```lua
  o.bind("SUPER + I", "Island", "omarchy-shell island toggle")
  ```

IPC (`omarchy-shell island <método>`):

| Método        | Efecto                                          |
|---------------|-------------------------------------------------|
| `toggle`      | Fija la isla abierta / cerrada                  |
| `open`        | La fija abierta                                 |
| `close`       | La cierra                                       |
| `peek`        | Muestra el vistazo                              |
| `timer`       | Inicia / pausa el temporizador                  |
| `focus <min>` | Duración del enfoque (con el temporizador parado, 5–120) |
| `state`       | JSON con el estado actual                       |

## Ajustes

Opcionales, en la entrada del plugin en `~/.config/omarchy/shell.json`:

```json
{ "id": "thealejandro.island", "language": "es" }
```

| Clave      | Valores    | Por defecto   |
|------------|------------|---------------|
| `language` | `en`, `es` | según `LANG`  |

## Desarrollo

Clónalo donde quieras y enlázalo en la carpeta de plugins:

```bash
git clone https://github.com/thealejandro/omarchy-island.git
ln -s "$PWD/omarchy-island" ~/.config/omarchy/plugins/thealejandro.island
omarchy-shell shell rescanPlugins
omarchy plugin enable thealejandro.island
```

Quickshell guarda en caché el QML compilado: después de editar el código,
ejecuta `omarchy restart shell` para ver los cambios.

| Archivo            | Qué hace                                               |
|--------------------|--------------------------------------------------------|
| `Service.qml`      | Estado: reproductor MPRIS, temporizador, vistazo, ajustes, IPC |
| `IslandModel.js`   | Funciones puras: elegir reproductor, reloj, textos, mes |
| `IslandWindow.qml` | Capa por pantalla, máscara de entrada, hover, animación |
| `IslandShape.qml`  | La silueta con orejas cóncavas                         |
| `PeekView.qml`     | El vistazo de una línea                                |
| `OpenView.qml`     | Disposición de la isla abierta                         |
| `MediaCard.qml`    | Portada, canción, barra de progreso y controles        |
| `CalendarView.qml` | Calendario del mes                                     |
| `TimerRow.qml`     | Temporizador de enfoque                                |
| `IslandButton.qml` | Botón redondo con icono                                |
| `Equalizer.qml`    | Barritas animadas mientras suena música                |

Ideas y fallos, en los issues. Inspirado en
[Nusa](https://rama2555.github.io/nusa/) para Windows y en el notch de
[Ambxst](https://github.com/Axenide/Ambxst); no se ha tomado código de
ninguno.

## Licencia

MIT
