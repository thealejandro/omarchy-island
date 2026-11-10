<p align="left">
  <a href="https://thealejandro.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/marca/logotipo-oscuro.svg">
      <img src="docs/marca/logotipo-claro.svg" alt="thealejandro" height="32">
    </picture>
  </a>
</p>

# omarchy-island

[English](README.md)

Una isla dinámica para [Omarchy](https://omarchy.org) que sale de la barra
de Omarchy, justo donde está el reloj, en el borde que tenga la barra.
Muestra lo que suena, te da controles de música, tus notificaciones (si
las quieres ahí), un calendario con tu propia agenda, la hora en el mundo, el clima con pronóstico de 3 días, un
temporizador de enfoque, acciones rápidas y los toggles de Omarchy, y se
aparta cuando terminas. Todo se configura desde una página de ajustes dentro
de la isla.

Es un plugin normal del shell de Omarchy: corre dentro de `omarchy-shell`,
toma los colores, la fuente y el acento de tu tema y no reemplaza ni modifica
nada de Omarchy. Si lo quitas, tu escritorio queda exactamente como estaba.

<p align="center">
  <img src="docs/island-demo.gif" alt="La isla asomándose con la canción actual, abriéndose desde el reloj y pasando por Calendario con la agenda, Notificaciones y Ajustes" width="600">
</p>

| Inicio | Vistazo |
| --- | --- |
| ![Isla abierta en Inicio: próximos eventos, música, temporizador y dock](docs/island-open.png) | ![Vistazo con la canción actual](docs/island-peek.png) |
| **Calendario** | **Notificaciones** |
| ![Calendario con la agenda de hoy y los próximos días](docs/island-calendar.png) | ![Historial de notificaciones con Silencio y Borrar historial](docs/island-notifications.png) |
| **Ajustes** | **Barra a la izquierda** |
| ![Ajustes, pestaña General](docs/island-settings.png) | ![La isla colgando de una barra a la izquierda](docs/island-bar-left.png) |
| **Barra abajo** | |
| ![La isla subiendo desde una barra abajo](docs/island-bar-bottom.png) | |

## Funciones

- **Vistazo**: cuando cambia la canción, termina el temporizador, conectas o
  desconectas el portátil, o la batería baja (20 % y 10 %), una línea baja de
  la barra unos segundos.
- **Agentes de IA**: Claude Code, Codex y otros avisan a la isla, con
  hooks, cuando trabajan, piden permiso o terminan: una fila en Inicio, un
  vistazo y un punto junto al reloj. Ver [Agentes de IA](#agentes-de-ia).
- **Volumen y brillo en la isla** (opcional): el OSD de Omarchy (volumen,
  brillo, micrófono, luz del teclado, teclas multimedia) aparece como un
  vistazo con su barra de nivel en lugar de una tarjeta flotante. Ver `osd`
  abajo.
- **Notificaciones en la isla** (opcional, desactivado por defecto): las
  notificaciones nuevas bajan de la barra como un vistazo, y una página
  Notificaciones las agrupa por app, con sus botones, respuestas y
  enlaces, más un historial de 24 horas. Las tarjetas de Omarchy se
  apartan mientras está activo, así que nada sale dos veces. Ver
  [Notificaciones](#notificaciones).
- **Letra sincronizada** (opcional, desactivada por defecto): la línea que
  suena se ve en la isla abierta, o se queda bajo el reloj mientras suena
  una canción. Se obtiene de lrclib.net. Ver [Letra](#letra).
- **Abrir**: pasa el ratón por el reloj de la barra, o justo debajo (o usa
  un atajo), y se abre la isla: portada,
  reproductor, título, artista, barra de progreso que puedes pulsar o
  arrastrar, anterior / reproducir / siguiente.
- **Páginas**: la isla abierta tiene cinco, en las pestañas de su esquina:
  Inicio, Calendario, Mundo, Notificaciones y Ajustes.
- **Inicio**: lo que suena junto a lo próximo (los tres siguientes eventos
  de tu agenda, con un botón para añadir uno), las últimas notificaciones
  cuando la isla las tiene, el temporizador y el dock.
- **Calendario y agenda**: el mes con hoy resaltado y un punto en los días
  con algo. Las flechas o la rueda del ratón cambian de mes; pulsa el título
  para volver a hoy. La página Calendario lista el día elegido: tus eventos
  (añadir, editar, repetir semanal/mensual/anual), los recordatorios de
  Omarchy y lo que viene en la próxima semana. La isla te avisa antes de un
  evento (vistazo y notificación) o lo pasa a `omarchy reminder` con un
  clic. Ver [Agenda](#agenda).
- **Mundo y clima**: las ciudades del widget elsewhen de Omarchy en la
  página Mundo (hora, día, diferencia con aquí), más el clima con
  pronóstico de 3 días. Un clic en la hora mundial o el clima de la barra
  la abre. Ver
  [Hora mundial y clima](#hora-mundial-y-clima).
- **Página de ajustes**: todos los ajustes de abajo, con interruptores,
  opciones y botones +/−. Los cambios se guardan al momento en `shell.json`
  y lo que cambies por línea de comandos aparece ahí en vivo.
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
- **Español, inglés y portugués**: sigue `LANG`, o elígelo en `shell.json`.
- **Sigue tu tema**: cambia en vivo con `omarchy theme set` (colores, fuente
  y acento). La isla tiene esquinas redondeadas con "orejas" cóncavas que se
  funden con la barra, y un contorno muy sutil (algo más marcado en temas
  claros). Un tema puede ajustarlo en su `shell.toml`: `[island]` `border`,
  `border-alpha`, `border-width`.
- **Movimiento**: la isla se abre con un muelle, y las páginas y los
  vistazos entran deslizándose un instante; solo transformaciones y
  opacidad. `animations: false` lo quita todo.
- **Ligera**: sin procesos extra. La música llega por MPRIS (Spotify,
  navegadores, mpv…); lo único que se consulta solo es el clima, cada 20
  minutos, los husos de la hora mundial mientras la isla está abierta y
  `icsSources` y `agendaCommand` si los configuras. Cerrada, la isla no guarda páginas,
  portadas ni iconos. Ver [Rendimiento](#rendimiento).

## Novedades

La versión 0.6.0 puede mostrar las notificaciones en la isla (vistazo,
página con grupos e historial; desactivado por defecto), rehace Inicio con
lo próximo de tu agenda en lugar de los relojes mundiales, hace que el reloj
abra el Calendario y la hora mundial y el clima de la barra abran la página
Mundo, añade animaciones ligeras y gasta menos memoria con la isla cerrada.
El detalle está en [CHANGELOG.md](CHANGELOG.md) (en inglés).

## Requisitos

Omarchy 4 o posterior (`omarchy-shell` basado en Quickshell; probado en
4.0.0.alpha). La barra puede estar en cualquier borde: arriba, abajo,
izquierda o derecha. Con una versión de Omarchy con la que la isla no se ha
probado, su página de ajustes lo indica; lo que no encuentre pasa a un
comportamiento más simple en lugar de romperse (ver
[Compatibilidad](#compatibilidad)).

## Instalación

```bash
omarchy plugin add https://github.com/thealejandro/island.git --enable
```

Después ponla en la barra junto al reloj. Ahí no ocupa sitio; sirve para
ocultar los indicadores que la isla ya lleva, y es como la isla encuentra el
reloj en la barra para colgar exactamente de él:

```bash
omarchy bar put thealejandro.island --after omarchy.clock
```

Para quitarla cuando quieras. Si la isla tenía las notificaciones o el
OSD de Omarchy, Omarchy los recupera solo a los pocos segundos (también si
solo la deshabilitas):

```bash
omarchy plugin remove thealejandro.island
```

Si borraste la carpeta de la isla a mano con el shell parado, devuélveselos
tú:

```bash
omarchy plugin enable omarchy.notifications; omarchy plugin enable omarchy.osd
```

## Uso

- Pasa el ratón por el reloj de la barra, o por la franja fina junto a él,
  para abrir la isla; al alejarte se cierra. Se abre sobre el reloj esté
  donde esté (centro, izquierda, derecha) y en el borde que tenga la barra.
- Pulsa el reloj para abrir la isla en la página Calendario; vuelve a
  pulsar para cerrarla. Con `clockClick` ese clic puede solo fijar la isla
  abierta en su página o dejar el reloj al calendario de Omarchy. El clic
  derecho sigue cambiando el formato del reloj y el central sigue abriendo
  el menú de zonas horarias.
- Pulsa la hora mundial (elsewhen) o el clima de Omarchy en la barra para
  abrir la isla en la página Mundo en lugar de sus paneles (`barClicks`).
  El clic central sigue abriendo su panel y el derecho hace lo mismo que en
  el widget (actualizar, o el clima como notificación). Pasar el ratón por
  ellos no abre la isla.
- Con la isla abierta, los demás widgets de la barra siguen siendo
  clicables y sus paneles se abren encima.
- Cerrada, la isla muestra una pequeña muesca bajo el reloj, su propia
  silueta en el color de la barra, para saber que está ahí (`idleNotch`:
  `off` la quita; `hour`, `day` y `battery` añaden una línea fina que crece
  con la hora, el día o el nivel de batería; `icon` pone un icono pequeño
  dentro, ver más abajo). Cede ante los puntos, los vistazos y el
  temporizador, no recibe clics, y con `hoverOpen` posar el puntero en
  ella abre la isla.
- Elige página en las pestañas de la esquina. Un clic en un día del
  calendario pequeño abre la página Calendario en ese día.
- Con la isla fijada también funciona el teclado: haz clic en ella (o
  ejecuta `omarchy-shell island keyboard`) y Tab recorre sus pestañas,
  botones y ajustes con un anillo visible, Espacio o Enter pulsan, ←/→
  cambian de página, sección u opción y suben o bajan números, y Esc la
  cierra. Abrirla al pasar el puntero nunca le quita el teclado a tu
  ventana. Todos los botones de icono tienen nombre para lectores de
  pantalla. Se desactiva con `keyboard: false`.
- Para abrirla con una tecla, en `~/.config/hypr/bindings.lua`. `SUPER + D`
  está libre en Omarchy (`SUPER + I` cambia el idioma del teclado);
  `keyboard` abre la isla lista para moverse con Tab, y la misma tecla o Esc
  la cierra:

  ```lua
  o.bind("SUPER + D", "Isla", "omarchy-shell island keyboard")
  ```

  Con `keyboard: false`, usa `omarchy-shell island toggle`, que solo la fija
  abierta o cerrada. El plugin nunca edita tu configuración de Hyprland por
  su cuenta.

IPC (`omarchy-shell island <método>`):

| Método        | Efecto                                          |
|---------------|-------------------------------------------------|
| `toggle`      | Fija la isla abierta / cerrada                  |
| `open`        | La fija abierta                                 |
| `close`       | La cierra                                       |
| `keyboard`    | La fija y toma el teclado (Tab para moverse, Esc para salir); otra vez lo suelta |
| `page <nombre>` | La abre en `home`, `calendar`, `world`, `notifications` o `settings` |
| `notifications <island\|off>` | Toma las notificaciones de Omarchy o se las devuelve (ver [Notificaciones](#notificaciones)) |
| `peek`        | Muestra el vistazo                              |
| `timer`       | Inicia / pausa el temporizador                  |
| `focus <min>` | Duración del enfoque (con el temporizador parado, 5–120) |
| `event <fecha> <hora> <título>` | Añade un evento (`2026-11-20 09:30 "Dentista"`; hora `""` para todo el día) |
| `events <fecha>` | JSON con los eventos de ese día (`""` para hoy) |
| `anchor <pantalla> <centro> <largo>` | De dónde colgar, para barras que no se lo pueden decir a la isla |
| `anchors`     | JSON con el borde de la barra y las anclas en uso |
| `state`       | JSON con el estado actual                       |
| `agent <nombre> <estado>`, `agents`… | Estado de agentes de IA, ver [Agentes de IA](#agentes-de-ia) |

## Ajustes

Todo está en la página Ajustes de la isla. Los valores viven en la entrada
de la isla en `~/.config/omarchy/shell.json` (en el layout de la barra, o en
`plugins` si no la pusiste en la barra); solo se escribe lo que cambias:

```json
{ "id": "thealejandro.island", "language": "es", "defaultPage": "calendar" }
```

O por línea de comandos: `omarchy bar set thealejandro.island toggles both`
(listas y `null` con `--json`). **Restablecer**, en la sección Sistema,
quita solo las claves propias de la isla.

| Clave | Valores | Por defecto | Qué hace |
|-------|---------|-------------|----------|
| `language` | `en`, `es`, `pt` | según `LANG` | Idioma de la isla |
| `defaultPage` | `home`, `calendar`, `world` | `home` | Página con la que se abre |
| `calendarFirst` | `true`, `false` | `false` | Lo próximo a la izquierda en Inicio |
| `showMedia`, `showTimer`, `showDock` | `true`, `false` | `true` | Partes de la página Inicio |
| `lyrics` | `true`, `false` | `false` | Letra sincronizada en la tarjeta de reproducción, de lrclib.net (necesita `showMedia`); ver [Letra](#letra) |
| `lyricsPeek` | `true`, `false` | `false` | La línea actual se queda bajo el reloj, como vistazo, con la isla cerrada mientras suena una canción (necesita `lyrics`) |
| `animations` | `true`, `false` | `true` | Muelle y movimiento de páginas y vistazos |
| `idleNotch` | `off`, `notch`, `icon`, `hour`, `day`, `battery` | `notch` | Con la isla cerrada: nada, una ceja de la barra bajo el reloj, una muesca más alta con un icono pequeño (`idleIcon`), o la ceja y una línea fina que se llena con la hora, el día (de 15 en 15 minutos) o el nivel de batería (roja con 20% o menos sin cargar; ceja normal sin batería). El icono no cede ante los puntos; cuelgan debajo |
| `idleIcon` | `glasses`, `isle`, `prompt`, `capsule`, `braces`, `mark`, `archipelago`, `swell`, `channel`, `fold`, o un nombre de archivo | `glasses` | El icono de `idleNotch: icon` (hay diez incluidos; `glasses`, el predeterminado, sale del logo del autor; los seis primeros tienen una pieza propia que se mueve). El tuyo: pon un `.svg` o `.png` (hasta 256 KiB, PNG hasta 1024 px) en `~/.config/omarchy/island/icons` y escribe su nombre, por ejemplo `rocket.svg`. Si no sirve, se ve la muesca normal |
| `idleIconTint` | `auto`, `on`, `off` | `auto` | Pinta el icono con el color de acento a través de su forma. `auto`: los iconos incluidos, los SVG de un solo color y los archivos `…-symbolic`; los PNG se ven tal cual |
| `idleMotion` | `true`, `false` | `true` | De vez en cuando (cada 20 a 45 s, o cuando llega algo nuevo) el icono mira a un lado. Entre medias no corre nada, y con `animations` apagado tampoco |
| `openWidth` | 420–800 | `560` | Ancho de la isla abierta |
| `blur` | `true`, `false` | `true` | Desenfoque detrás de la isla con la barra de cristal |
| `clock` | `true`, `false` | `true` | Abrir al pasar sobre el reloj (`false`: solo la franja del borde) |
| `clockClick` | `pin`, `calendar`, `omarchy` | `calendar` | Clic en el reloj: fijar la isla, abrirla en el calendario o el calendario de Omarchy |
| `barClicks` | `true`, `false` | `true` | Un clic en la hora mundial o el clima de la barra abre la página Mundo |
| `anchor` | `clock`, `widget`, `center` | `clock` | Colgar del reloj de Omarchy, del sitio de la isla en la barra o del centro |
| `clockWidth` | 60–400 | `120` | Zona de hover si no se encuentra el reloj |
| `hoverOpen` | `true`, `false` | `true` | Abrir al pasar el puntero |
| `keyboard` | `true`, `false` | `true` | Recorrer la isla fijada con Tab, Espacio y las flechas |
| `hoverDelay` | 0–600 ms | `120` | Espera antes de abrir al pasar |
| `peekDuration` | 1000–10000 ms | `3500` | Duración de un vistazo |
| `toggles` | `island`, `both`, `bar` | `island` | Dónde viven los toggles de Omarchy |
| `widgetIcon` | `true`, `false` | `false` | Un botón propio pequeño en la barra |
| `units` | `metric`, `imperial` | `metric` | Unidades del clima |
| `forecast` | `true`, `false` | `true` | Pronóstico de 3 días en Mundo |
| `hour24` | `true`, `false` | el de elsewhen, si no el del idioma | Hora mundial en 24 h |
| `cities` | `[{"name","zone"}]` | las de elsewhen | Lista de ciudades propia |
| `weekStart` | 1 (lun) – 7 (dom) | el del idioma | Primer día de la semana |
| `reminderLead` | 0–120 min | `10` | Aviso antes de un evento |
| `icsSources` | carpetas, archivos `.ics`, enlaces `https://` (separados por comas) | `""` | Calendarios que mostrar (Google, Outlook, iCloud, vdirsyncer…) |
| `icsRefresh` | 5–240 min | `15` | Cada cuánto se vuelven a leer esos calendarios |
| `agendaCommand` | comando | `""` | Eventos extra de otro calendario |
| `osd` | `true`, `false` | `false` | Mostrar el OSD de Omarchy en la isla |
| `battery` | `true`, `false` | `true` | Vistazos de carga y batería baja |
| `privacyDots` | `true`, `false` | `true` | Punto naranja mientras una app usa el micrófono, verde mientras una usa la cámara |
| `privacyPeek` | `true`, `false` | `true` | Un vistazo con el nombre de la app cuando empieza a usarlos |
| `agents` | `true`, `false` | `true` | Muestra agentes de IA (Claude Code, Codex…) en Inicio, ver [Agentes de IA](#agentes-de-ia) |
| `agentPeek` | `true`, `false` | `true` | Un vistazo cuando un agente te necesita, termina o falla |
| `agentPeekDuration` | 2000–15000 ms | `5000` | Cuánto dura (el doble si te necesita) |
| `agentDot` | `true`, `false` | `true` | Punto rojo a la izquierda del reloj mientras un agente te necesita |
| `agentStale` | 10–480 min | `120` | Un agente trabajando o esperando que no da señales pasa a inactivo |
| `notifications` | `off`, `island` | `off` | Mostrar las notificaciones en la isla en lugar de las tarjetas de Omarchy |
| `notifyBody` | `true`, `false` | `true` | El texto en el vistazo (`false`: solo app y título) |
| `notifyPeekDuration` | 2000–15000 ms | `6000` | Duración del vistazo de una notificación |
| `notifyHistory` | `true`, `false` | `true` | Guardar las notificaciones cerradas 24 h |
| `notifyHistoryLimit` | 20–500 | `100` | Notificaciones cerradas que se guardan |
| `notifyBadge` | `true`, `false` | `true` | Punto de no leídas bajo el reloj y en la pestaña |
| `notifyKeyboardIdle` | 3000–30000 | `10000` | Milisegundos sin pulsar nada antes de que el modo teclado suelte el teclado |

`toggles` decide dónde viven los toggles rápidos de Omarchy:

- `island`: en la isla; la barra muestra solo los indicadores activos.
- `both`: en la isla, y la barra sigue mostrándolos todos al pasar el ratón
  por su centro (el comportamiento de Omarchy).
- `bar`: solo en la barra; el dock de la isla mantiene las acciones y el
  clima.

`osd` hace que la isla muestre el OSD de Omarchy. Solo un plugin puede ser
el dueño, así que la tarjeta de Omarchy tiene que apartarse. El botón
**Usar como OSD** de la sección Sistema hace los dos pasos; a mano:

```bash
omarchy plugin disable omarchy.osd
omarchy bar set thealejandro.island osd true
```

Para volver: `omarchy bar set thealejandro.island osd false` y
`omarchy plugin enable omarchy.osd` (o el mismo botón otra vez).

`battery: false` silencia los vistazos de carga y batería baja. En equipos
sin batería nunca aparecen.

## Notificaciones

Desactivado por defecto. Con `notifications: island` la isla es tu
servidor de notificaciones:

- Una notificación nueva baja de la barra como un vistazo con el icono de
  la app, el título y (con `notifyBody`) el texto, durante
  `notifyPeekDuration` o el tiempo que pida quien la envía. Las críticas se
  quedan hasta que las atiendes. Al ir con el ratón al vistazo se abre la
  página Notificaciones.
- La página Notificaciones las agrupa por app, la más nueva primero (las
  del navegador, por sitio, así WhatsApp Web y GitHub no comparten un
  grupo de Chromium, y los avisos de Omarchy, cada uno por su lado), con
  sus botones, respuesta en línea cuando la app la ofrece, un enlace
  seguro para abrir y un botón de cerrar por notificación o por grupo.
- Al pulsar una notificación se hace lo que pidió quien la envió (su
  acción por defecto). Si no pidió nada, el clic te lleva a quien la
  envió: su ventana (por el proceso que la mandó o por la clase de la
  ventana de la app), la pestaña del sitio en las del navegador o, si no
  hay, la portada del sitio o el enlace del texto. La flecha bajo el botón
  de cerrar muestra el texto completo.
- El atajo de Omarchy para manejar las notificaciones con el teclado
  (Super+Ctrl+,) abre la isla en su página Notificaciones y toma el
  teclado: ↑/↓ (o J/K, Tab) para moverse entre tarjetas, Enter hace lo
  mismo que un clic, Supr cierra, 1–9 pulsan un botón, C copia el código
  de la tarjeta y Esc sale. Suelta el teclado tras `notifyKeyboardIdle`
  sin pulsar nada (10 s) o al cerrarse la isla.
- Cuando una notificación trae un código de un solo uso («Tu código de
  verificación es 482913»), su tarjeta tiene un botón **Copiar**. El
  código va al portapapeles marcado como sensible, así los gestores de
  portapapeles no lo guardan en su historial, y se borra al minuto si
  sigue ahí. Los códigos nunca se guardan con el historial.
  **Silencio** es el mismo «no molestar» de Omarchy (se guarda en el
  propio `~/.local/state/omarchy/notifications/quiet.json` de Omarchy), así
  que el indicador de la barra y el toggle del dock coinciden.
- **Anteriores** lista las notificaciones cerradas de las últimas 24 horas
  (como mucho `notifyHistoryLimit`), guardadas en
  `~/.local/state/omarchy/island/notifications.json`, legible solo por ti.
  No se guardan las imágenes enviadas por el bus ni los botones.
  **Limpiar historial** lo vacía.
- Inicio muestra las dos últimas, y un punto bajo el reloj y en la pestaña
  avisa de que hay algo sin leer (`notifyBadge`).

Solo un programa puede recibir las notificaciones del escritorio, así que
las tarjetas de Omarchy tienen que apartarse, igual que con el OSD. El
interruptor de la sección Sistema de Ajustes hace los dos pasos; desde una
terminal:

```bash
omarchy-shell island notifications island   # la isla las toma
omarchy-shell island notifications off      # Omarchy las recupera
```

Para que la isla las tome hacen falta las dos cosas: su ajuste **y**
`omarchy.notifications` deshabilitado. Si el plugin de Omarchy se vuelve a
habilitar por cualquier vía (`omarchy plugin enable omarchy.notifications`,
un restablecimiento, una actualización de Omarchy), la isla se aparta al
momento y Omarchy muestra las tarjetas; Ajustes indica entonces que la isla
está a la espera. Si se quita la isla o se restablece su ajuste mientras
las tenía, Omarchy las recupera la próxima vez que arranque el shell. El IPC
`notifications` de Omarchy (`dismissAll`, `toggleDnd`, `invokeLast`…) sigue
funcionando con la isla, así que su menú y sus atajos también.

Las notificaciones que están en pantalla cuando se reinicia el shell se
guardan como cerradas en el historial; sus botones se pierden.

## Agenda

Tus eventos se guardan en `~/.local/state/omarchy/island/events.json`,
aparte de `shell.json` (son datos, no ajustes). La isla lo escribe de forma
atómica y recoge lo que edites a mano:

```json
{
  "version": 1,
  "events": [
    { "id": "e1", "date": "2026-11-20", "time": "09:30", "title": "Dentista",
      "repeat": "none", "remind": null }
  ]
}
```

`time` es `""` para un evento de todo el día; `repeat` es `none`, `weekly`,
`monthly` o `yearly`; `remind` son minutos antes, o `null` para usar
`reminderLead`. En ese momento la isla se asoma y manda una notificación.
La campana de un evento de hoy lo pasa a los recordatorios de Omarchy
(`omarchy reminder <minutos> "<título>"`), y los recordatorios activos de
Omarchy aparecen en hoy.

**Tus calendarios (Google, Outlook, iCloud, CalDAV).** `icsSources` recibe
una lista, separada por comas, de calendarios en formato iCalendar, que se
muestran de solo lectura junto a tus eventos (y avisan igual que ellos).
Cada uno puede ser:

- **Un enlace iCal privado** (`https://…` o `webcal://…`), que se descarga
  cada `icsRefresh` minutos. Google Calendar: *Configuración › tu
  calendario › Integrar el calendario › Dirección secreta en formato
  iCal*. Outlook.com / Microsoft 365: *Configuración › Calendario ›
  Calendarios compartidos › Publicar un calendario* (el enlace ICS).
  iCloud: comparte el calendario como *Calendario público*. Fastmail,
  Proton y Nextcloud también lo ofrecen.
- **Una carpeta** de archivos `.ics`, como las que
  [vdirsyncer](https://github.com/pimutils/vdirsyncer) mantiene
  sincronizadas desde cualquier servidor CalDAV (Nextcloud, Radicale,
  iCloud, Fastmail, Google con su soporte CalDAV) con `fileext = ".ics"`.
- **Un archivo `.ics`** (una exportación, un calendario de festivos…).

```bash
omarchy bar set thealejandro.island icsSources "~/.local/share/vdirsyncer/calendars, https://calendar.google.com/calendar/ical/…/basic.ics"
```

Trata un enlace iCal privado como una contraseña: quien lo tenga puede leer
ese calendario. Se guarda en tu `shell.json`, así que no publiques ese
archivo en unos dotfiles públicos (o usa la carpeta de vdirsyncer, cuyas
credenciales viven en su propia configuración). La isla descarga los
enlaces solo por HTTPS y nunca los escribe en el log. Entiende eventos
sueltos y repetidos (diarios, semanales, mensuales, anuales, con
excepciones y ocurrencias movidas), de todo el día y de varios días, y
husos horarios, y muestra un año antes y después de hoy (hasta 2000
eventos). Lo que un calendario haga y no entienda se omite en lugar de
mostrarse mal.

**Otros calendarios.** `agendaCommand` ejecuta un comando al arrancar la
isla y cada 15 minutos; cada línea que imprima como
`AAAA-MM-DD<TAB>HH:MM<TAB>título` (hora vacía para todo el día) aparece como
un evento de solo lectura. Por ejemplo con
[khal](https://github.com/pimutils/khal) (comprueba las opciones de formato
de tu versión de khal):

```bash
omarchy bar set thealejandro.island agendaCommand "khal list --format '{start-date}\t{start-time}\t{title}' --day-format '' today 14d"
```

## Hora mundial y clima

La página Mundo usa las ciudades del widget elsewhen de Omarchy, leídas de su entrada en `shell.json`, así mantienes una sola
lista. Pon `cities` para darle a la isla la suya (`Nombre|Zona/Ciudad, …`
en la página de ajustes). La isla calcula las horas a partir del desfase de
cada zona (`TZ=zona date`), así que no necesita elsewhen en la barra. Sin
ciudades, la página Mundo muestra solo el clima.

El clima es el mismo parte de wttr.in de la píldora del dock, para el lugar
configurado en el widget de clima de Omarchy (o por IP), cada 20 minutos.
**Abrir pronóstico** abre el panel del clima de Omarchy (si el widget está
en la barra) y **Cambiar ubicación** su selector de lugar, o el archivo que
lee.

## Letra

Desactivada por defecto. Con `lyrics: true` (Ajustes › General › Letra
sincronizada), la tarjeta de reproducción muestra la línea que suena y la
siguiente, al compás de la canción. Con `lyricsPeek: true` la línea actual
también se queda bajo el reloj, como un vistazo, mientras suena una canción
y la isla está cerrada (cualquier otro vistazo pasa por delante y la letra
vuelve después).

Viene de [lrclib.net](https://lrclib.net), un servicio comunitario gratuito
sin cuenta ni clave. **Qué sale de tu equipo**: el artista, el título, el
álbum y la duración de la canción, en una petición a lrclib.net, junto con el
`User-Agent` `omarchy-island/<versión> (https://github.com/thealejandro/island)`.
Nunca la ruta del archivo ni el nombre del reproductor. **Cuándo**: al
empezar una canción nueva con la isla abierta (o con `lyricsPeek`), y al
abrirla sobre una canción que aún no ha buscado; como mucho tres peticiones
por canción, una tras otra. Con el ajuste apagado, que es el valor por
defecto, no se pide nada ni se carga nada. Los nombres con «(Remastered)»,
«feat. …» y similares se vuelven a probar sin ellos, y luego con una
búsqueda.

**Qué se guarda**: cada respuesta, encontrada o no, en
`~/.cache/omarchy/island/lyrics` (o bajo `$XDG_CACHE_HOME`): hasta 400
canciones, las encontradas 90 días y las que faltan un día. Una canción en
caché se muestra sin volver a preguntar. Para borrarla:
`rm -r ~/.cache/omarchy/island/lyrics`.

Solo se muestran letras con tiempos; las que no los tienen y las
instrumentales no muestran nada. Pueden faltar, ser otras o ir algo
desfasadas en algunas canciones (lrclib compara la duración con dos
segundos de margen, y algunos reproductores, como un navegador con un
vídeo, la informan mal). Las letras las escriben quienes colaboran en
lrclib.net, y la tarjeta dice de dónde vienen.

La línea sigue la posición real del reproductor: se notan los saltos, las
pausas y los cambios de velocidad, y no hay sondeo (ver
[Rendimiento](#rendimiento)). `omarchy-shell island state` muestra `lyrics`
(`off`, `idle`, `loading`, `synced`, `plain`, `instrumental`, `none`) y la
`lyric` actual.

## Micrófono y cámara

Mientras una app graba del micrófono, aparece un punto naranja justo
debajo de la barra, junto al reloj (y en el botón propio de la isla en la
barra, si lo usas); mientras una usa la cámara, uno verde. Cuando una app
empieza, un vistazo dice cuál: *Micrófono en uso · Firefox*. `omarchy-shell
island state` también las lista (`mic`, `camera`).

El micrófono se lee de los propios eventos de PipeWire, sin consultas
periódicas: cuentan las capturas; los medidores de nivel (el panel de audio
de Omarchy, pavucontrol, cava…), las capturas silenciadas y las
grabaciones de lo que suena (el monitor de una salida) no. Las cámaras que
una app abre a través de PipeWire aparecen igual; para una abierta
directamente por `/dev/video*` la isla pregunta a `fuser` cada 15 segundos
(5 mientras se ve), y solo en equipos con cámara. Los procesos de otros
usuarios no se ven sin root. Los temas pueden cambiar el color de los
puntos en `shell.toml`: `[island]` `mic`, `camera` (y `mic-alpha`,
`camera-alpha`). `privacyDots: false` lo apaga todo y no carga nada.

## Agentes de IA

Claude Code, Codex, Gemini CLI, Aider y cualquier otra cosa que pueda
ejecutar un comando en un evento pueden decirle a la isla qué están
haciendo. En Inicio, la isla lista hasta dos agentes (el más reciente
primero) con su estado: *Trabajando*, *Te necesita*, *Terminó*, *Falló* o
*Inactivo*, y "+N" si hay más. Un vistazo baja de la barra cuando uno pide
permiso (dura el doble), termina o falla, y un punto rojo queda a la
izquierda del reloj mientras uno te espera. Pulsar la fila de un agente
sube su ventana cuando la isla sabe cuál es (ver *Subir la ventana*).
`omarchy-shell island agents` los lista en JSON, y `state` también
(`agents`, `agentsWaiting`).

No se consulta nada: la isla solo oye lo que envían los hooks. Los agentes
que terminan salen de la lista a los 5 minutos, los que fallan a los 10, y
uno que estaba trabajando o esperando pero no da señales en `agentStale`
minutos (2 h por defecto) pasa a inactivo durante un minuto y desaparece.
`agents: false` lo apaga todo.

Pensado también para quien lanza muchas sesiones a la vez: la isla sigue
hasta 64. Pasado ese número salen primero las terminadas, inactivas o
fallidas más antiguas y luego la que lleva más tiempo trabajando; **una que
espera tu permiso nunca sale para hacer sitio** (solo un tope de 128, para
un hook que se desboque). Si terminan muchas a la vez, cada vistazo
sustituye al anterior en lugar de encolarse, y el punto rojo sigue mientras
alguna espere. `agent-hook.sh` cuesta unas decenas de milisegundos por
evento: los eventos que la isla no usa se quedan en un `jq`, y a Hyprland
solo se le pregunta por la ventana cuando una sesión empieza a trabajar o
pide permiso.

**El directorio de trabajo nunca se guarda ni se muestra entero**: la isla
solo se queda con el nombre de su última carpeta (`~/Work/mi-app` se ve como
*mi-app*), y al texto que llega de un agente se le quitan los caracteres de
control y de escape y se recorta a unas decenas de caracteres.

### Claude Code

Añade esto a `~/.claude/settings.json` (únelo con los `hooks` que ya tengas):

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh claude", "async": true, "timeout": 5 }] }],
    "Notification": [{ "matcher": "permission_prompt|elicitation_dialog|agent_needs_input", "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh claude", "async": true, "timeout": 5 }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh claude", "async": true, "timeout": 5 }] }],
    "StopFailure": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh claude", "async": true, "timeout": 5 }] }],
    "SessionEnd": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh claude", "async": true, "timeout": 5 }] }]
  }
}
```

El script lee el JSON del hook por stdin y deduce el estado del evento. Usa
`Notification` (como arriba) **o** un hook `PermissionRequest` para «te
necesita», no los dos. Un hook que falla o tarda nunca retiene a Claude
Code: el script sale en silencio si falta `jq` u `omarchy-shell`. Un agente
muestra *Te necesita* hasta que vuelve a trabajar (tu siguiente mensaje) o
termina.

### Codex

Añade esto a `~/.codex/hooks.json` (puede que haya que activar los hooks en
la configuración de Codex; mira su documentación):

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh codex", "timeout": 5 }] }],
    "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh codex", "timeout": 5 }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh codex", "timeout": 5 }] }],
    "SessionEnd": [{ "hooks": [{ "type": "command", "command": "$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh codex", "timeout": 5 }] }]
  }
}
```

Como alternativa, el ajuste `notify` de Codex en `~/.codex/config.toml`
ejecuta un programa al terminar un turno. Es una suposición, no algo que la
documentación de Codex detalle: el script trata el JSON que Codex añade como
opaco, lee `thread-id` y `cwd` si están y usa el estado que le des:

```toml
notify = ["$HOME/.config/omarchy/plugins/thealejandro.island/scripts/agent-hook.sh", "codex", "done"]
```

### Cualquier otro

```bash
omarchy-shell -q island agent aider working
omarchy-shell -q island agent aider done
omarchy-shell -q island agent aider end        # olvidarlo
```

| Método | Efecto |
|--------|--------|
| `agent <nombre> <estado>` | Una sesión por agente; estado `working`, `waiting`, `done`, `error`, `idle` (o `end` para quitarla) |
| `agentSession <nombre> <id> <estado> <cwd> <mensaje>` | Lo mismo con id de sesión, la carpeta en que trabaja y un mensaje corto (`""` para omitirlos) |
| `agentWindow <nombre> <id> <pid> <address>` | La ventana en que corre, para el clic |
| `agentClear <nombre> <id>` | Olvida una sesión, todas las de un agente (`id` `""`) o todas (`nombre` `""`) |
| `agentJson '<json>'` | `{ "agent", "id", "state", "cwd", "message", "pid", "address" }`, hasta 4 KiB |
| `agents` | JSON con las sesiones |

Todos piden todos los argumentos (usa `""`) y responden `ok`, `invalid` u
`off` (con `agents` apagado). Los nombres `start`, `busy`, `permission`,
`stop`, `fail`, `exit`… valen como alias de los estados.

### Subir la ventana

La isla sube una ventana por su address de Hyprland, o por el pid del
proceso de la ventana, solo cuando pulsas, y nunca adivinándola por un
título o una clase. `agent-hook.sh` encuentra la terminal en la que corre el
agente subiendo por sus procesos padre. En tus propios scripts, pasa `pid` o
`address` a `agentWindow`; la address de la ventana en la que estás es
`hyprctl activewindow -j | jq -r .address`. Dentro de tmux o zellij puede
que la terminal no sea un ancestro del agente, y entonces la fila no se
puede pulsar.

## Rendimiento

La isla corre dentro de `omarchy-shell`, así que lo que cuesta es lo que
crece el proceso del shell. Medido en Omarchy 4.0.0.alpha (una pantalla
1920×1080 a 120 Hz), como PSS del shell, tres pasadas intercaladas:

| | Shell sin la isla | 0.5.0 | 0.6.0 |
|---|---|---|---|
| Isla cerrada | ~273 MB | 285 MB | 278 MB |
| Isla abierta (Inicio) | | 289 MB | 289 MB |
| Tras visitar todas las páginas y cerrar | | 286 MB | 289 MB* |

\* La memoria que usaron las vistas no siempre se devuelve al sistema al
momento; no crece al abrir y cerrar más (cinco ciclos seguidos se quedan
planos). Con `notifications: island` el shell pesa unos MB menos (cerrada
~270 MB), porque se descarga el plugin de notificaciones de Omarchy.

- Cerrada, la isla no guarda páginas, portadas ni iconos: la vista abierta
  y el vistazo se crean al mostrarse y se sueltan un momento después de
  irse.
- La vista abierta se empieza a crear en segundo plano en cuanto el puntero
  llega al reloj, así que está lista cuando la isla se abre: el primer
  fotograma visible llega unos 24 ms después de empezar a abrirse (0.5.0:
  25 ms), sin ningún fotograma de más de 25 ms al abrir o cerrar.
- La posición de la canción solo avanza mientras suena música y la isla
  está abierta.
- La muesca de la isla cerrada es un elemento estático. El hilo de la hora
  (`idleNotch: hour`) despierta una vez por minuto y el del día (`day`)
  cada cuarto de hora, solo con la isla cerrada y sin nada más bajo el
  reloj. El de la batería no usa temporizador: sigue el nivel de batería.
- La letra, si está activada, usa un solo temporizador que espera a la
  siguiente línea (no sondea) y lee la posición del reproductor cada vez
  que salta. Con la isla cerrada no hace nada, salvo con `lyricsPeek`.

## Compatibilidad

La isla solo usa lo que Omarchy ofrece a un plugin, y lo comprueba en lugar
de darlo por hecho:

- El borde y el tamaño de la barra vienen de la API de barra para plugins;
  sin ella, de `shell.json` y del tema. Con la barra oculta la isla se abre
  en el borde de la pantalla.
- El reloj se busca entre los widgets de la barra (`omarchy.clock`). Si una
  barra futura cambia eso, la isla cuelga de su propio sitio en la barra
  (como `anchor: widget`) o del centro.
- Los ajustes se guardan con `updateEntryInline` de la API de plugins, o con
  `omarchy bar set` si falta.
- Nunca escribe archivos de Omarchy; solo su entrada en `shell.json` (y
  `disabledPlugins` mediante `omarchy plugin` cuando le pides usar el OSD o
  las notificaciones), el estado de «no molestar» de Omarchy en el mismo
  archivo que usa Omarchy, y `~/.local/state/omarchy/island/`.
- Los clics en la hora mundial y el clima de la barra no necesitan nada de
  esos widgets: la isla los cubre en la barra y usa su IPC pública para el
  clic central y el derecho. `barClicks: false` los deja como estaban.

### Problemas frecuentes

- *La isla se abre en el centro de la barra, no sobre el reloj*: pon la isla
  en la barra (`omarchy bar put thealejandro.island --after
  omarchy.clock`) o revisa `omarchy-shell island anchors`.
- *`osd: true` pero no hay OSD en la isla*: `omarchy.osd` de Omarchy sigue
  habilitado; usa el botón de la sección Sistema.
- *No hay ciudades en la página Mundo*: elsewhen no tiene ciudades (o está
  deshabilitado) y `cities` está vacío.
- *Las notificaciones están activas pero siguen saliendo las tarjetas de
  Omarchy*: `omarchy.notifications` vuelve a estar habilitado (Ajustes dice
  que la isla está a la espera); usa el interruptor de la sección Sistema o
  `omarchy-shell island notifications island`.
- *No llega ninguna notificación*: mira `omarchy-shell notifications probe`
  (`server` dice quién las tiene). `omarchy-shell island notifications off`
  siempre se las devuelve a Omarchy.
- *Al pulsar el clima o la hora mundial se abre el panel de Omarchy*: pon
  la isla en la barra (los encuentra desde ahí) y deja `barClicks` activo.
- *Una barra de terceros*: sin la API de barra de Omarchy la isla usa
  `shell.json`; la barra puede indicar el reloj con `omarchy-shell island
  anchor <pantalla> <centro> <largo>`.

## Desarrollo

Clónalo donde quieras y enlázalo en la carpeta de plugins:

```bash
git clone https://github.com/thealejandro/island.git
ln -s "$PWD/island" ~/.config/omarchy/plugins/thealejandro.island
omarchy-shell shell rescanPlugins
omarchy plugin enable thealejandro.island
```

Quickshell guarda en caché el QML compilado: después de editar el código,
ejecuta `omarchy restart shell` para ver los cambios.

La lógica pura (los archivos `.js`) tiene pruebas que corren en Node 22 o
posterior, sin instalar nada; la CI las pasa en cada pull request:

```bash
node --test 'test/*.test.mjs'
```

| Archivo            | Qué hace                                               |
|--------------------|--------------------------------------------------------|
| `Service.qml`      | Estado: reproductor MPRIS, temporizador, toggles, clima, mundo, agenda, OSD, batería, modo de notificaciones, ajustes, anclas, IPC |
| `BarWidget.qml`    | Entrada en la barra: oculta indicadores duplicados, encuentra el reloj, la hora mundial y el clima, botón opcional |
| `NotificationService.qml` | El servidor de notificaciones de la isla: filas, vistazos, acciones, no molestar, historial |
| `NotificationModel.js`, `NotificationSecurity.js` | Funciones puras: filas, grupos e historial; comprobación de enlaces, comandos e imágenes |
| `IslandSettings.js` | Esquema de ajustes: valores por defecto, normalización, guardado, esquema del manifest |
| `IslandGeometry.js` | Bordes de la barra, posición a lo largo de ella, búsqueda del reloj |
| `IslandIdle.js`    | Muesca de la isla cerrada y hilo de la hora: qué dibujar, largo del hilo, siguiente minuto |
| `IslandModel.js`   | Funciones puras: elegir reproductor, reloj, textos, mes, clima, horas del mundo |
| `AgendaModel.js`   | Funciones puras de eventos: repeticiones, próximos, avisos, líneas externas |
| `IcsModel.js`, `scripts/ics-fetch.sh` | Calendarios iCalendar: lectura, repeticiones, husos; descarga de carpetas, archivos y enlaces |
| `LyricsService.qml`, `LyricsModel.js`, `scripts/lyrics-fetch.sh` | Letra sincronizada: búsqueda en lrclib.net y caché, lectura de LRC, seguir la canción |
| `PrivacyService.qml`, `PrivacyModel.js` | Quién usa el micrófono o la cámara (PipeWire, fuser) |
| `AgentModel.js`, `scripts/agent-hook.sh` | Estado de agentes de IA: sesiones, limpieza, caducidad; el script de hooks para Claude Code, Codex y otros |
| `IslandWindow.qml` | Capa por pantalla, máscara de entrada, hover, clics de la barra, animación |
| `IslandShape.qml`  | La silueta con orejas cóncavas, girada al borde de la barra |
| `PeekView.qml`     | El vistazo de una línea: canción, temporizador, OSD, batería, evento, agente de IA o notificación |
| `OpenView.qml`     | Cabecera, pestañas y página actual                     |
| `HomePage.qml`, `TodayCard.qml` | La página de inicio y su tarjeta «Lo próximo» |
| `NotificationsPage.qml`, `NotificationGroup.qml`, `NotificationCard.qml`, `NotificationIcon.qml` | Página Notificaciones |
| `CalendarPage.qml`, `AgendaList.qml`, `EventEditor.qml` | Página de calendario y agenda |
| `WorldPage.qml`    | Página Mundo                                           |
| `SettingsView.qml`, `PageTabs.qml` | Página de ajustes y pestañas         |
| `IslandToggle.qml`, `IslandSegment.qml`, `IslandStepper.qml`, `IslandField.qml` | Controles de los ajustes |
| `MediaCard.qml`    | Portada, canción, barra de progreso y controles        |
| `CalendarView.qml` | Calendario del mes                                     |
| `TimerRow.qml`     | Temporizador de enfoque                                |
| `Dock.qml`         | Fila de toggles, acciones y clima                      |
| `IslandButton.qml`, `IslandPill.qml` | Botón redondo con icono y píldora de texto |
| `Glyph.qml`        | Icono centrado por su tinta, no por su caja            |
| `Equalizer.qml`    | Barritas animadas mientras suena música                |

Ideas y fallos, en los issues. Inspirado en
[Nusa](https://rama2555.github.io/nusa/) para Windows y en el notch de
[Ambxst](https://github.com/Axenide/Ambxst); no se ha tomado código de
ninguno.

## Licencia

MIT

---

Hecho por [thealejandro](https://thealejandro.dev).
