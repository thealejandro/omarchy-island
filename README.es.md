# omarchy-island

[English](README.md)

Una isla dinámica para [Omarchy](https://omarchy.org) que sale del centro de
la barra. Muestra lo que suena, te da controles de música, el calendario del
mes, un temporizador de enfoque, el clima, acciones rápidas y los toggles de
Omarchy, y se aparta cuando terminas.

Es un plugin normal del shell de Omarchy: corre dentro de `omarchy-shell`,
toma los colores, la fuente y el acento de tu tema y no reemplaza ni modifica
nada de Omarchy. Si lo quitas, tu escritorio queda exactamente como estaba.

## Funciones

- **Vistazo**: cuando cambia la canción, termina el temporizador, conectas o
  desconectas el portátil, o la batería baja (20 % y 10 %), una línea baja de
  la barra unos segundos.
- **Volumen y brillo en la isla** (opcional): el OSD de Omarchy (volumen,
  brillo, micrófono, luz del teclado, teclas multimedia) aparece como un
  vistazo con su barra de nivel en lugar de una tarjeta flotante. Ver `osd`
  abajo.
- **Abrir**: pasa el ratón por el reloj de la barra, o justo debajo (o usa
  un atajo), y se abre la isla: portada,
  reproductor, título, artista, barra de progreso que puedes pulsar o
  arrastrar, anterior / reproducir / siguiente.
- **Calendario**: el mes actual con hoy resaltado. Las flechas o la rueda del
  ratón cambian de mes; pulsa el título para volver a hoy.
- **Temporizador de enfoque**: Pomodoro (25 / 5 por defecto) con una
  notificación al terminar cada fase. Usa la rueda sobre el temporizador
  para cambiar la duración del enfoque (5–120 min).
- **Dock**: una fila fina en la parte de abajo.
  - Toggles rápidos: luz nocturna, silenciar notificaciones, mantener
    despierto, grabar pantalla y recordatorios, los mismos interruptores que
    los indicadores de la barra de Omarchy. Un botón iluminado está activo.
  - Acciones: captura de pantalla, cuentagotas, historial del portapapeles y
    selector de emojis. La isla se aparta antes de que empiecen.
  - Clima: icono, temperatura y estado del lugar configurado en el widget de
    clima de Omarchy (o por IP), desde wttr.in. Púlsalo para ver el
    pronóstico de Omarchy.
  - Pasa el ratón por un botón para ver su nombre.
- **Sin toggles duplicados**: con la isla en la barra (ver abajo), pasar el
  ratón por el centro de la barra ya no muestra los indicadores inactivos de
  Omarchy; la barra deja solo los que están activos. Configurable.
- **Español e inglés**: sigue `LANG`, o elígelo en `shell.json`.
- **Sigue tu tema**: cambia en vivo con `omarchy theme set` (colores, fuente
  y acento). La isla tiene esquinas redondeadas con "orejas" cóncavas que se
  funden con la barra.
- **Ligera**: sin procesos extra. La música llega por MPRIS (Spotify,
  navegadores, mpv…); lo único que se consulta solo es el clima, cada 20
  minutos.

## Requisitos

Omarchy 4 o posterior (`omarchy-shell` basado en Quickshell) con la barra
arriba.

## Instalación

```bash
omarchy plugin add https://github.com/thealejandro/omarchy-island.git --enable
```

Luego ponla en el centro de la barra, junto al reloj. No ocupa sitio ahí; es
lo que le permite ocultar los indicadores que ya lleva la isla:

```bash
omarchy bar put thealejandro.island --after omarchy.clock
```

Para quitarlo:

```bash
omarchy plugin remove thealejandro.island
```

## Uso

- Pasa el ratón por el reloj de la barra, o por la franja fina justo debajo,
  para abrir la isla; al alejarte se cierra.
- Pulsa el reloj para dejarla fija (su calendario sustituye al de Omarchy);
  vuelve a pulsarlo para cerrarla. El clic derecho sigue cambiando el formato
  del reloj y el central sigue abriendo el menú de zona horaria.
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

Opcionales, en la entrada de la isla en `~/.config/omarchy/shell.json` (en
la disposición de la barra, o en `plugins` si no la pusiste en la barra):

```json
{ "id": "thealejandro.island", "language": "es", "toggles": "island" }
```

O desde la terminal: `omarchy bar set thealejandro.island toggles both`.

| Clave      | Valores                  | Por defecto  |
|------------|--------------------------|--------------|
| `language` | `en`, `es`               | según `LANG` |
| `clock`    | `true`, `false`          | `true`       |
| `toggles`  | `island`, `both`, `bar`  | `island`     |
| `units`    | `metric`, `imperial`     | `metric`     |
| `osd`      | `true`, `false`          | `false`      |
| `battery`  | `true`, `false`          | `true`       |

`toggles` decide dónde van los toggles rápidos de Omarchy:

- `island`: en la isla; la barra muestra solo los indicadores activos.
- `both`: en la isla, y la barra los sigue mostrando todos al pasar el ratón
  por su centro (como hace Omarchy).
- `bar`: solo en la barra; el dock de la isla se queda con las acciones y el
  clima.

`osd` hace que la isla muestre el OSD de Omarchy. Solo un plugin puede
tenerlo, así que primero apaga la tarjeta de Omarchy y luego actívalo:

```bash
omarchy plugin disable omarchy.osd
omarchy bar set thealejandro.island osd true
```

Para volver: `omarchy bar set thealejandro.island osd false` y
`omarchy plugin enable omarchy.osd`.

`battery: false` silencia los vistazos de carga y batería baja. En un
equipo sin batería nunca aparecen.

Con `"clock": false` la isla deja el reloj en paz: se abre solo desde el
borde superior y un clic en el reloj vuelve a abrir el calendario de Omarchy.

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
| `Service.qml`      | Estado: reproductor MPRIS, temporizador, toggles, clima, OSD, batería, ajustes, IPC |
| `BarWidget.qml`    | Entrada invisible en la barra que oculta indicadores duplicados |
| `IslandModel.js`   | Funciones puras: elegir reproductor, reloj, textos, mes, clima |
| `IslandWindow.qml` | Capa por pantalla, máscara de entrada, hover, animación |
| `IslandShape.qml`  | La silueta con orejas cóncavas                         |
| `PeekView.qml`     | El vistazo de una línea: canción, temporizador, OSD o batería |
| `OpenView.qml`     | Disposición de la isla abierta                         |
| `MediaCard.qml`    | Portada, canción, barra de progreso y controles        |
| `CalendarView.qml` | Calendario del mes                                     |
| `TimerRow.qml`     | Temporizador de enfoque                                |
| `Dock.qml`         | Fila de toggles, acciones y clima                      |
| `IslandButton.qml` | Botón redondo con icono                                |
| `Glyph.qml`        | Icono centrado por su tinta, no por su caja            |
| `Equalizer.qml`    | Barritas animadas mientras suena música                |

Ideas y fallos, en los issues. Inspirado en
[Nusa](https://rama2555.github.io/nusa/) para Windows y en el notch de
[Ambxst](https://github.com/Axenide/Ambxst); no se ha tomado código de
ninguno.

## Licencia

MIT
