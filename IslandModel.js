.pragma library

// Prefer whatever is playing right now; otherwise the first player that has
// a track loaded, so a paused song stays reachable.
function pickPlayer(players) {
  if (!players || players.length === 0) return null
  var withTrack = null
  for (var i = 0; i < players.length; i++) {
    var p = players[i]
    if (!p) continue
    if (p.isPlaying) return p
    if (!withTrack && (p.trackTitle || p.trackArtist)) withTrack = p
  }
  return withTrack
}

function clock(seconds) {
  var s = Math.max(0, Math.floor(seconds || 0))
  var m = Math.floor(s / 60)
  var r = s % 60
  return m + ":" + (r < 10 ? "0" : "") + r
}

// ------------------------------------------------------------- language

var STRINGS = {
  en: {
    nothingPlaying: "Nothing playing",
    nothingPlayingHint: "Play something in Spotify, a browser or mpv",
    focus: "Focus",
    break: "Break",
    focusDone: "Focus done",
    takeBreak: "Take a %1 minute break.",
    breakOver: "Break over",
    readyAgain: "Ready for another round?",
    today: "Today",
    home_next: "Next",
    home_nothing: "Nothing coming up.",
    home_add: "Add an event",
    minutes: "%1 min",
    scrollToChange: "Scroll to change",
    nightLight: "Night light",
    doNotDisturb: "Silence",
    stayAwake: "Stay awake",
    recording: "Record",
    stopRecording: "Stop",
    reminders: "Reminder",
    screenshot: "Screenshot",
    colorPicker: "Color picker",
    clipboard: "Clipboard",
    emoji: "Emoji",
    weather: "Weather",
    charging: "Charging",
    onBattery: "On battery",
    batteryLow: "Battery low",
    fullyCharged: "Fully charged",
    page_home: "Home",
    page_calendar: "Calendar",
    page_world: "World",
    page_settings: "Settings",
    page_missing: "This page could not be loaded.",
    sec_general: "General",
    sec_look: "Look",
    sec_open: "Opening",
    sec_bar: "Bar",
    sec_world: "World",
    sec_agenda: "Agenda",
    sec_system: "System",
    set_language: "Language",
    set_defaultPage: "Opens on",
    set_calendarFirst: "Agenda first on Home",
    set_showMedia: "Now playing",
    set_lyrics: "Synced lyrics",
    hint_lyrics: "From lrclib.net: the artist, title, album and length of each track are sent there; cached on this computer",
    set_lyricsPeek: "Lyrics glance while a song plays",
    hint_lyricsPeek: "The current line stays under the clock while the island is closed",
    set_showTimer: "Focus timer",
    set_showDock: "Toggles and launchers",
    set_openWidth: "Width",
    set_blur: "Blur behind a glassy bar",
    set_clock: "Open by hovering the clock",
    set_clockClick: "Clicking the clock",
    set_barClicks: "World clock and weather open the island",
    hint_barClicks: "Left click opens the World page instead of Omarchy's popup; middle click still opens it",
    set_anchor: "Hang from",
    set_clockWidth: "Clock zone when not found",
    set_hoverOpen: "Open on hover",
    set_hoverDelay: "Hover delay",
    set_peekDuration: "Glance duration",
    set_toggles: "Quick toggles",
    set_widgetIcon: "Island button on the bar",
    set_units: "Units",
    set_forecast: "3-day forecast",
    set_hour24: "Clock format",
    set_cities: "Cities (Name|Zone, …)",
    set_weekStart: "Week starts on",
    set_reminderLead: "Remind me before",
    set_icsSources: "Calendars (.ics folders, files or links)",
    set_icsRefresh: "Refresh calendars every",
    set_agendaCommand: "External agenda command",
    set_osd: "Show Omarchy's OSD here",
    set_battery: "Battery glances",
    set_privacyDots: "Microphone and camera dots",
    set_privacyPeek: "Glance when an app starts using them",
    hint_privacyDots: "Orange while an app uses the microphone, green while one uses the camera",
    micInUse: "Microphone in use · %1",
    cameraInUse: "Camera in use · %1",
    micCameraInUse: "Microphone and camera · %1",
    set_agents: "AI agent status",
    hint_agents: "Claude Code, Codex and others report through hooks (see the README); shown on Home",
    set_agentPeek: "Glance when an agent needs you or finishes",
    set_agentPeekDuration: "Agent glance duration",
    set_agentDot: "Dot when an agent needs you",
    hint_agentDot: "Just past the bar, to the left of the clock",
    set_agentStale: "Forget a stuck agent after",
    hint_agentStale: "A working or waiting agent nobody has heard from becomes idle",
    agent_working: "Working",
    agent_waiting: "Needs you",
    agent_done: "Finished",
    agent_error: "Failed",
    agent_idle: "Idle",
    agentWaiting: "%1 needs your permission",
    agentDone: "%1 finished",
    agentError: "%1 failed",
    opt_auto: "Auto",
    opt_home: "Home",
    opt_calendar: "Calendar",
    opt_world: "World",
    opt_pin: "Pin island",
    opt_omarchy: "Omarchy panel",
    opt_clock: "Clock",
    opt_widget: "Its widget",
    opt_center: "Center",
    opt_island: "Island",
    opt_both: "Both",
    opt_bar: "Bar",
    opt_metric: "°C",
    opt_imperial: "°F",
    opt_h12: "12 h",
    opt_h24: "24 h",
    opt_mon: "Mon",
    opt_sat: "Sat",
    opt_sun: "Sun",
    hint_cities: "Empty uses Elsewhen's cities",
    hint_icsSources: "Comma-separated: a vdirsyncer folder, an .ics file or a private iCal link (https://)",
    hint_agendaCommand: "Prints YYYY-MM-DD<TAB>HH:MM<TAB>title lines",
    act_osdOn: "Use as OSD",
    act_osdOff: "Give the OSD back",
    act_place: "Next to the clock",
    act_forecast: "Open forecast",
    act_weatherPlace: "Change location",
    tomorrow: "Tomorrow",
    yesterday: "Yesterday",
    world_empty: "No cities yet. Add them to Omarchy's elsewhen widget or under Settings › World.",
    allDay: "All day",
    eventIn: "in %1 min",
    eventNow: "now",
    ev_title: "What's happening?",
    ev_repeat: "Repeat",
    ev_remind: "Remind me (minutes before)",
    ev_save: "Save",
    ev_cancel: "Cancel",
    ev_delete: "Delete",
    ev_invalid: "Needs a title and a time like 09:30.",
    ev_reminded: "Omarchy will remind you.",
    ev_tooLate: "Too late for a reminder.",
    ev_none: "Nothing on this day.",
    ev_omarchyReminders: "Omarchy reminders",
    ev_next: "Next days",
    rep_none: "Once",
    rep_weekly: "Weekly",
    rep_monthly: "Monthly",
    rep_yearly: "Yearly",
    act_reset: "Reset",
    act_openConfig: "Open shell.json",
    set_keyboard: "Keyboard navigation",
    hint_keyboard: "Pinned: Tab moves · Space presses · ←→ choose · Esc closes",
    a11y_pages: "Pages",
    a11y_sections: "Sections",
    a11y_add: "Add event",
    a11y_remind: "Remind me",
    a11y_close: "Close",
    a11y_closeGroup: "Close group",
    a11y_expand: "Show more",
    a11y_collapse: "Show less",
    a11y_fold: "Fold group",
    a11y_unfold: "Unfold group",
    a11y_reset: "Reset timer",
    a11y_play: "Play",
    a11y_pause: "Pause",
    a11y_prev: "Previous",
    a11y_next: "Next",
    a11y_prevMonth: "Previous month",
    a11y_nextMonth: "Next month",
    a11y_agentRow: "Go to the window of %1",
    a11y_lyric: "Lyric: %1",
    lyricsBy: "lyrics by lrclib.net",
    note_noEntry: "The island has no entry in shell.json, so changes last until restart. Use “Next to the clock”.",
    note_version: "Tested with Omarchy 4.x; this is %1.",
    note_reset: "Reset removes only the island's own keys.",
    page_notifications: "Notifications",
    sec_notify: "Notifications",
    set_animations: "Animations",
    set_idleNotch: "Closed island",
    hint_idleNotch: "A sliver of the bar under the clock so you know the island is there; a small icon or a thread can fill it with the hour, the day or the battery",
    opt_notch: "Notch",
    opt_icon: "Icon",
    opt_on: "On",
    set_idleIcon: "Icon in the notch",
    hint_idleIcon: "Eleven built in, or your own .svg or .png in ~/.config/omarchy/island/icons",
    set_idleIconTint: "Tint the icon",
    hint_idleIconTint: "Auto paints one-colour SVGs and -symbolic files with the accent",
    set_idleMotion: "Icon motion",
    hint_idleMotion: "Now and then it glances to the side; nothing runs in between",
    opt_hour: "Hour thread",
    opt_day: "Day thread",
    opt_battery: "Battery thread",
    set_notifications: "Show notifications in the island",
    set_notifyBody: "Show the text in the glance",
    set_notifyPeekDuration: "Notification glance",
    set_notifyHistory: "Keep a history",
    set_notifyHistoryLimit: "History size",
    set_notifyBadge: "Unread dot",
    set_notifyKeyboardIdle: "Keyboard mode timeout",
    opt_off: "Off",
    hint_notifications: "Replaces Omarchy's notification cards",
    hint_notifyBody: "Off shows only the app and the title",
    hint_notifyHistory: "Saved for 24 h on this computer",
    hint_notifyKeyboardIdle: "↑↓ move · Enter open · Del close · 1–9 actions · C copy code · Esc leave",
    act_notifyOn: "Show notifications here",
    act_notifyOff: "Give notifications back",
    note_notifyStandby: "Omarchy's notifications plugin is on again, so the island stands by. Turn the setting off, or disable it with: omarchy plugin disable omarchy.notifications",
    note_notifyFailed: "The island could not receive notifications, so Omarchy shows them again.",
    note_notifyOff: "Notifications are shown by Omarchy. Turn on “Show notifications in the island” under Settings › System.",
    nt_empty: "All caught up",
    nt_emptyHint: "New notifications show up here and as a glance.",
    nt_markRead: "Mark read",
    nt_dismissAll: "Close all",
    nt_clear: "Clear history",
    nt_dnd: "Silence",
    nt_earlier: "Earlier",
    nt_unread: "%1 unread",
    nt_now: "now",
    nt_minAgo: "%1 min",
    nt_hAgo: "%1 h",
    nt_reply: "Reply…",
    nt_openLink: "Open link",
    nt_copyCode: "Copy %1",
    nt_copied: "Copied",
    nt_close: "Close",
    nt_more: "+%1",
    nt_recent: "Notifications",
    credit: "%1 · by thealejandro"
  },
  es: {
    nothingPlaying: "Nada sonando",
    nothingPlayingHint: "Pon algo en Spotify, el navegador o mpv",
    focus: "Enfoque",
    break: "Descanso",
    focusDone: "Enfoque terminado",
    takeBreak: "Tómate un descanso de %1 minutos.",
    breakOver: "Descanso terminado",
    readyAgain: "¿Otra ronda?",
    today: "Hoy",
    home_next: "Lo próximo",
    home_nothing: "Nada en la agenda.",
    home_add: "Añadir evento",
    minutes: "%1 min",
    scrollToChange: "Rueda para cambiar",
    nightLight: "Luz nocturna",
    doNotDisturb: "Silencio",
    stayAwake: "Despierto",
    recording: "Grabar",
    stopRecording: "Detener",
    reminders: "Recordatorio",
    screenshot: "Captura",
    colorPicker: "Cuentagotas",
    clipboard: "Portapapeles",
    emoji: "Emoji",
    weather: "Clima",
    charging: "Cargando",
    onBattery: "Con batería",
    batteryLow: "Batería baja",
    fullyCharged: "Carga completa",
    page_home: "Inicio",
    page_calendar: "Calendario",
    page_world: "Mundo",
    page_settings: "Ajustes",
    page_missing: "No se pudo cargar esta página.",
    sec_general: "General",
    sec_look: "Aspecto",
    sec_open: "Apertura",
    sec_bar: "Barra",
    sec_world: "Mundo",
    sec_agenda: "Agenda",
    sec_system: "Sistema",
    set_language: "Idioma",
    set_defaultPage: "Abre en",
    set_calendarFirst: "Agenda primero en Inicio",
    set_showMedia: "Reproducción",
    set_lyrics: "Letra sincronizada",
    hint_lyrics: "De lrclib.net: se le envían artista, título, álbum y duración de cada pista; se guarda en este equipo",
    set_lyricsPeek: "Vistazo con la letra mientras suena",
    hint_lyricsPeek: "La línea actual se queda bajo el reloj con la isla cerrada",
    set_showTimer: "Temporizador de enfoque",
    set_showDock: "Interruptores y accesos",
    set_openWidth: "Anchura",
    set_blur: "Desenfoque con barra de cristal",
    set_clock: "Abrir al pasar sobre el reloj",
    set_clockClick: "Click en el reloj",
    set_barClicks: "Hora mundial y clima abren la isla",
    hint_barClicks: "El click izquierdo abre la página Mundo en vez del panel de Omarchy; el central sigue abriéndolo",
    set_anchor: "Colgar de",
    set_clockWidth: "Zona del reloj si no se encuentra",
    set_hoverOpen: "Abrir al pasar el puntero",
    set_hoverDelay: "Retardo al pasar",
    set_peekDuration: "Duración del vistazo",
    set_toggles: "Interruptores rápidos",
    set_widgetIcon: "Botón de la isla en la barra",
    set_units: "Unidades",
    set_forecast: "Pronóstico de 3 días",
    set_hour24: "Formato de hora",
    set_cities: "Ciudades (Nombre|Zona, …)",
    set_weekStart: "La semana empieza en",
    set_reminderLead: "Avisarme antes",
    set_icsSources: "Calendarios (carpetas, archivos o enlaces .ics)",
    set_icsRefresh: "Actualizar calendarios cada",
    set_agendaCommand: "Comando de agenda externa",
    set_osd: "Mostrar aquí el OSD de Omarchy",
    set_battery: "Avisos de batería",
    set_privacyDots: "Puntos de micrófono y cámara",
    set_privacyPeek: "Vistazo cuando una app empieza a usarlos",
    hint_privacyDots: "Naranja mientras una app usa el micrófono, verde mientras una usa la cámara",
    micInUse: "Micrófono en uso · %1",
    cameraInUse: "Cámara en uso · %1",
    micCameraInUse: "Micrófono y cámara · %1",
    set_agents: "Estado de agentes de IA",
    hint_agents: "Claude Code, Codex y otros avisan con hooks (mira el README); se ve en Inicio",
    set_agentPeek: "Vistazo cuando un agente te necesita o termina",
    set_agentPeekDuration: "Duración del vistazo de agentes",
    set_agentDot: "Punto cuando un agente te necesita",
    hint_agentDot: "Justo debajo de la barra, a la izquierda del reloj",
    set_agentStale: "Olvidar un agente colgado tras",
    hint_agentStale: "Un agente trabajando o esperando que no da señales pasa a inactivo",
    agent_working: "Trabajando",
    agent_waiting: "Te necesita",
    agent_done: "Terminó",
    agent_error: "Falló",
    agent_idle: "Inactivo",
    agentWaiting: "%1 pide permiso",
    agentDone: "%1 ha terminado",
    agentError: "%1 ha fallado",
    opt_auto: "Auto",
    opt_home: "Inicio",
    opt_calendar: "Calendario",
    opt_world: "Mundo",
    opt_pin: "Fijar isla",
    opt_omarchy: "Panel de Omarchy",
    opt_clock: "Reloj",
    opt_widget: "Su widget",
    opt_center: "Centro",
    opt_island: "Isla",
    opt_both: "Ambos",
    opt_bar: "Barra",
    opt_metric: "°C",
    opt_imperial: "°F",
    opt_h12: "12 h",
    opt_h24: "24 h",
    opt_mon: "Lun",
    opt_sat: "Sáb",
    opt_sun: "Dom",
    hint_cities: "Vacío usa las ciudades de Elsewhen",
    hint_icsSources: "Separados por comas: una carpeta de vdirsyncer, un archivo .ics o un enlace iCal privado (https://)",
    hint_agendaCommand: "Imprime líneas AAAA-MM-DD<TAB>HH:MM<TAB>título",
    act_osdOn: "Usar como OSD",
    act_osdOff: "Devolver el OSD",
    act_place: "Junto al reloj",
    act_forecast: "Abrir pronóstico",
    act_weatherPlace: "Cambiar ubicación",
    tomorrow: "Mañana",
    yesterday: "Ayer",
    world_empty: "Aún no hay ciudades. Añádelas en el widget elsewhen de Omarchy o en Ajustes › Mundo.",
    allDay: "Todo el día",
    eventIn: "en %1 min",
    eventNow: "ahora",
    ev_title: "¿Qué pasa?",
    ev_repeat: "Repetir",
    ev_remind: "Avisarme (minutos antes)",
    ev_save: "Guardar",
    ev_cancel: "Cancelar",
    ev_delete: "Borrar",
    ev_invalid: "Falta el título o una hora como 09:30.",
    ev_reminded: "Omarchy te avisará.",
    ev_tooLate: "Ya es tarde para avisar.",
    ev_none: "Nada para este día.",
    ev_omarchyReminders: "Recordatorios de Omarchy",
    ev_next: "Próximos días",
    rep_none: "Una vez",
    rep_weekly: "Semanal",
    rep_monthly: "Mensual",
    rep_yearly: "Anual",
    act_reset: "Restablecer",
    act_openConfig: "Abrir shell.json",
    set_keyboard: "Navegación con teclado",
    hint_keyboard: "Fijada: Tab avanza · Espacio pulsa · ←→ eligen · Esc cierra",
    a11y_pages: "Páginas",
    a11y_sections: "Secciones",
    a11y_add: "Añadir evento",
    a11y_remind: "Avisarme",
    a11y_close: "Cerrar",
    a11y_closeGroup: "Cerrar grupo",
    a11y_expand: "Mostrar más",
    a11y_collapse: "Mostrar menos",
    a11y_fold: "Plegar grupo",
    a11y_unfold: "Desplegar grupo",
    a11y_reset: "Reiniciar temporizador",
    a11y_play: "Reproducir",
    a11y_pause: "Pausar",
    a11y_prev: "Anterior",
    a11y_next: "Siguiente",
    a11y_prevMonth: "Mes anterior",
    a11y_nextMonth: "Mes siguiente",
    a11y_agentRow: "Ir a la ventana de %1",
    a11y_lyric: "Letra: %1",
    lyricsBy: "letra de lrclib.net",
    note_noEntry: "La isla no tiene entrada en shell.json: los cambios duran hasta reiniciar. Usa «Junto al reloj».",
    note_version: "Probado con Omarchy 4.x; esta es %1.",
    note_reset: "Restablecer solo quita las claves propias de la isla.",
    page_notifications: "Notificaciones",
    sec_notify: "Avisos",
    set_animations: "Animaciones",
    set_idleNotch: "Isla cerrada",
    hint_idleNotch: "Una ceja del color de la barra bajo el reloj para saber que la isla está; un icono o un hilo pueden llenarla con la hora, el día o la batería",
    opt_notch: "Muesca",
    opt_icon: "Icono",
    opt_on: "Sí",
    set_idleIcon: "Icono en la muesca",
    hint_idleIcon: "Once incluidos, o tu propio .svg o .png en ~/.config/omarchy/island/icons",
    set_idleIconTint: "Teñir el icono",
    hint_idleIconTint: "Auto pinta con el acento los SVG de un color y los -symbolic",
    set_idleMotion: "Movimiento del icono",
    hint_idleMotion: "De vez en cuando mira a un lado; entre medias no corre nada",
    opt_hour: "Hilo de la hora",
    opt_day: "Hilo del día",
    opt_battery: "Hilo de la batería",
    set_notifications: "Mostrar las notificaciones en la isla",
    set_notifyBody: "Mostrar el texto en el vistazo",
    set_notifyPeekDuration: "Vistazo de notificación",
    set_notifyHistory: "Guardar historial",
    set_notifyHistoryLimit: "Tamaño del historial",
    set_notifyBadge: "Punto de no leídas",
    set_notifyKeyboardIdle: "Fin del modo teclado",
    opt_off: "No",
    hint_notifications: "Sustituye las tarjetas de Omarchy",
    hint_notifyBody: "Desactivado muestra solo la app y el título",
    hint_notifyHistory: "Se guarda 24 h en este equipo",
    hint_notifyKeyboardIdle: "↑↓ moverse · Enter abrir · Supr cerrar · 1–9 acciones · C copiar código · Esc salir",
    act_notifyOn: "Mostrar notificaciones aquí",
    act_notifyOff: "Devolver las notificaciones",
    note_notifyStandby: "El plugin de notificaciones de Omarchy vuelve a estar activo, así que la isla espera. Desactiva el ajuste o deshabilítalo con: omarchy plugin disable omarchy.notifications",
    note_notifyFailed: "La isla no pudo recibir notificaciones, así que Omarchy vuelve a mostrarlas.",
    note_notifyOff: "Las notificaciones las muestra Omarchy. Activa «Mostrar las notificaciones en la isla» en Ajustes › Sistema.",
    nt_empty: "Todo al día",
    nt_emptyHint: "Las notificaciones nuevas aparecen aquí y como vistazo.",
    nt_markRead: "Marcar leídas",
    nt_dismissAll: "Cerrar todas",
    nt_clear: "Limpiar historial",
    nt_dnd: "Silencio",
    nt_earlier: "Anteriores",
    nt_unread: "%1 sin leer",
    nt_now: "ahora",
    nt_minAgo: "%1 min",
    nt_hAgo: "%1 h",
    nt_reply: "Responder…",
    nt_openLink: "Abrir enlace",
    nt_copyCode: "Copiar %1",
    nt_copied: "Copiado",
    nt_close: "Cerrar",
    nt_more: "+%1",
    nt_recent: "Notificaciones",
    credit: "%1 · por thealejandro"
  }
}

// "es_ES.UTF-8", "es-MX" or "es" all resolve to "es"; anything we don't
// translate falls back to English.
function language(code) {
  var base = String(code || "").toLowerCase().split(/[_.@-]/)[0]
  return STRINGS[base] ? base : "en"
}

function t(lang, key, arg) {
  var table = STRINGS[lang] || STRINGS.en
  var s = table[key] !== undefined ? table[key] : (STRINGS.en[key] || key)
  return arg !== undefined ? s.replace("%1", arg) : s
}

// ------------------------------------------------------------- calendar

// 6 weeks x 7 days starting on the locale's first weekday, so every month
// has the same height and the island doesn't jump when paging through.
// firstDow uses Qt's numbering: 1 = Monday ... 7 = Sunday.
function monthGrid(year, month, firstDow, today) {
  var first = new Date(year, month, 1)
  var jsFirst = first.getDay()                 // 0 = Sunday
  var qtFirst = jsFirst === 0 ? 7 : jsFirst
  var lead = (qtFirst - firstDow + 7) % 7
  var cells = []
  for (var i = 0; i < 42; i++) {
    var d = new Date(year, month, 1 - lead + i)
    cells.push({
      day: d.getDate(),
      iso: d.getFullYear() + "-" + (d.getMonth() < 9 ? "0" : "") + (d.getMonth() + 1) + "-" + (d.getDate() < 10 ? "0" : "") + d.getDate(),
      inMonth: d.getMonth() === month,
      isToday: d.getFullYear() === today.getFullYear()
        && d.getMonth() === today.getMonth()
        && d.getDate() === today.getDate(),
      weekend: d.getDay() === 0 || d.getDay() === 6
    })
  }
  return cells
}

// Qt day numbers (1..7) for the header row, starting at firstDow.
function weekdayOrder(firstDow) {
  var out = []
  for (var i = 0; i < 7; i++) out.push(((firstDow - 1 + i) % 7) + 1)
  return out
}

// wttr.in's j1 report → { temp, code, desc, place }. lang picks wttr's
// translated description ("lang_es") when it has one. Anything malformed
// returns null so the caller keeps the last good report.
function parseWeather(raw, lang, imperial) {
  try {
    var data = JSON.parse(String(raw || ""))
    var now = data.current_condition[0]
    var translated = now["lang_" + lang]
    var desc = translated && translated[0] ? translated[0].value
      : (now.weatherDesc && now.weatherDesc[0] ? now.weatherDesc[0].value : "")
    var area = data.nearest_area && data.nearest_area[0] && data.nearest_area[0].areaName
      ? data.nearest_area[0].areaName[0].value : ""
    var temp = imperial ? now.temp_F : now.temp_C
    if (temp === undefined || temp === "") return null
    // The next days from the same report (wttr.in gives three), for the
    // world page. Midday's code stands for the day.
    var days = []
    var list = Array.isArray(data.weather) ? data.weather : []
    for (var i = 0; i < list.length && days.length < 3; i++) {
      var d = list[i]
      var hourly = Array.isArray(d.hourly) ? d.hourly : []
      var noon = hourly[Math.min(4, hourly.length - 1)] || {}
      var dayDesc = noon["lang_" + lang] && noon["lang_" + lang][0] ? noon["lang_" + lang][0].value
        : (noon.weatherDesc && noon.weatherDesc[0] ? noon.weatherDesc[0].value : "")
      var hi = imperial ? d.maxtempF : d.maxtempC, lo = imperial ? d.mintempF : d.mintempC
      if (!d.date || hi === undefined || lo === undefined) continue
      days.push({ date: String(d.date), max: hi + "°", min: lo + "°",
        code: parseInt(noon.weatherCode, 10), desc: String(dayDesc).trim() })
    }
    return { temp: temp + "°", code: parseInt(now.weatherCode, 10), desc: desc.trim(), place: area, days: days }
  } catch (e) {
    return null
  }
}

// ------------------------------------------------------------- world

// One shell for every zone: `TZ=zone date` prints its abbreviation and UTC
// offset, the same probe Omarchy's elsewhen uses. Times are then worked out
// here from the offset, so the probe only runs now and then (for DST).
function zoneProbeCommand(zones) {
  return ["bash", "-c", 'for z in "$@"; do TZ="$z" date "+$z|%Z|%z"; done', "bash"].concat(zones)
}

// Probe output → { "Asia/Tokyo": { abbr: "JST", offset: 540 }, ... }
function parseZoneProbe(text) {
  var map = {}
  var lines = String(text || "").split("\n")
  for (var i = 0; i < lines.length; i++) {
    var f = lines[i].split("|")
    if (f.length < 3) continue
    var m = /^([+-])(\d{2})(\d{2})$/.exec(f[2].trim())
    if (!m || f[0].trim() === "") continue
    var minutes = parseInt(m[2], 10) * 60 + parseInt(m[3], 10)
    map[f[0].trim()] = { abbr: f[1].trim(), offset: m[1] === "-" ? -minutes : minutes }
  }
  return map
}

function pad2(n) { return (n < 10 ? "0" : "") + n }

// A city's row at instant nowMs: wall-clock time, day relative to here,
// whether it's night there, and the hours it is ahead or behind.
function worldRow(city, probe, nowMs, hour24) {
  var p = probe ? probe[city.zone] : null
  if (!p) return null
  var there = new Date(Number(nowMs) + p.offset * 60000)
  var here = new Date(Number(nowMs))
  var hour = there.getUTCHours(), minute = there.getUTCMinutes()
  var delta = Math.round((Date.UTC(there.getUTCFullYear(), there.getUTCMonth(), there.getUTCDate())
    - Date.UTC(here.getFullYear(), here.getMonth(), here.getDate())) / 86400000)
  var diff = p.offset + here.getTimezoneOffset()
  var h12 = hour % 12 === 0 ? 12 : hour % 12
  return {
    name: city.name,
    zone: city.zone,
    abbr: p.abbr,
    time: hour24 ? pad2(hour) + ":" + pad2(minute) : h12 + ":" + pad2(minute),
    meridiem: hour24 ? "" : (hour < 12 ? "am" : "pm"),
    dayDelta: delta,
    night: hour < 6 || hour >= 19,
    diffMinutes: diff
  }
}

// "+3 h", "-5:30 h", "" for the same time as here.
function diffLabel(minutes) {
  if (!minutes) return ""
  var abs = Math.abs(minutes), h = Math.floor(abs / 60), m = abs % 60
  return (minutes < 0 ? "-" : "+") + h + (m ? ":" + pad2(m) : "") + " h"
}

// Nerd Font icon for a wttr.in weather code, the same set Omarchy's weather
// widget uses.
function weatherIcon(code, night) {
  switch (code) {
    case 113: return night ? "\ue32b" : "\ue30d"
    case 116: return night ? "\ue32e" : "\ue302"
    case 119: case 122: return "\ue33d"
    case 143: case 248: case 260: return night ? "\ue346" : "\ue313"
    case 176: case 263: case 353: return night ? "\ue333" : "\ue308"
    case 179: case 227: case 230: case 323: case 326: case 368: return night ? "\ue327" : "\ue30a"
    case 182: case 185: case 281: case 284: case 311: case 314:
    case 317: case 320: case 350: case 362: case 365: case 374: case 377: return "\ue3ad"
    case 200: case 386: case 389: case 392: case 395: return "\ue31d"
    case 266: case 293: case 296: case 299: case 302: case 305: case 308: case 356: case 359: return "\ue318"
    case 329: case 332: case 335: case 338: case 371: return "\ue31a"
    default: return "\ue33d"
  }
}

// ------------------------------------------------------------- osd

// Omarchy's OSD payload ({icon, message, value, max, progressText,
// duration}, as built by omarchy-osd) → what the peek shows. Same rules as
// Omarchy's own card: a value without a message is a progress bar.
function osdState(payloadJson) {
  var p
  try { p = JSON.parse(String(payloadJson || "{}")) } catch (e) { return null }
  var max = Math.max(1, parseInt(p.max || "100", 10) || 100)
  var raw = p.value === undefined || p.value === null ? "" : String(p.value)
  var value = parseInt(raw, 10)
  var message = String(p.message || "")
  var hasProgress = raw !== "" && !isNaN(value) && message === ""
  var percent = hasProgress ? Math.round(Math.max(0, Math.min(max, value)) * 100 / max) : -1
  var duration = parseInt(p.duration || "1200", 10)
  return {
    icon: osdIcon(p.icon, percent),
    message: message,
    hasProgress: hasProgress,
    progress: hasProgress ? percent / 100 : 0,
    label: hasProgress ? String(p.progressText || percent + "%") : "",
    duration: isNaN(duration) ? 1200 : Math.max(0, duration)
  }
}

// The icon names Omarchy's commands send, as Nerd Font glyphs. Unknown
// names are passed through, so a caller can send a glyph directly.
function osdIcon(name, percent) {
  var n = String(name || "").toLowerCase()
  var table = {
    "volume-muted": "\ueee8", "volume-mute": "\ueee8", "muted": "\ueee8", "mute": "\ueee8",
    "volume-low": "\uf026", "volume-medium": "\uf027", "volume-high": "\uf028", "volume": "\uf028",
    "microphone-muted": "\udb80\udf6d", "microphone-off": "\udb80\udf6d", "mic-muted": "\udb80\udf6d", "mic-off": "\udb80\udf6d",
    "microphone": "\udb80\udf6c", "mic": "\udb80\udf6c",
    "keyboard": "\udb80\udf0c", "brightness": "\udb80\udf79", "display": "\udb80\udf79",
    "touchpad": "\udb81\udff8", "touch": "\udb81\udf41", "touchscreen": "\udb81\udf41",
    "reboot": "\udb81\udf09", "restart": "\udb81\udf09",
    "shutdown": "\udb81\udc25", "power": "\udb81\udc25", "poweroff": "\udb81\udc25",
    "logout": "\udb80\udf43", "sign-out": "\udb80\udf43", "leave": "\udb80\udf43",
    "media": "\udb81\udf5a", "player": "\udb81\udf5a", "media-source": "\udb81\udf5a", "player-source": "\udb81\udf5a",
    "media-play": "\udb81\udc0a", "player-play": "\udb81\udc0a",
    "media-pause": "\udb80\udfe4", "player-pause": "\udb80\udfe4",
    "media-next": "\udb81\udcad", "player-next": "\udb81\udcad",
    "media-previous": "\udb81\udcae", "player-previous": "\udb81\udcae"
  }
  if (table[n]) return table[n]
  if (n.length > 0) return String(name)
  if (percent <= 0) return table["volume-muted"]
  if (percent <= 33) return table["volume-low"]
  if (percent <= 66) return table["volume-medium"]
  return table["volume-high"]
}

// ------------------------------------------------------------- battery

// Battery glyph for a 0..100 level, filled in tenths, with the bolt while
// charging.
function batteryIcon(level, charging) {
  if (charging) return "\udb80\udc84"
  var steps = ["\udb80\udc8e", "\udb80\udc7a", "\udb80\udc7b", "\udb80\udc7c", "\udb80\udc7d",
    "\udb80\udc7e", "\udb80\udc7f", "\udb80\udc80", "\udb80\udc81", "\udb80\udc82", "\udb80\udc79"]
  return steps[Math.max(0, Math.min(10, Math.round(level / 10)))]
}

// The arrow keys on a row of choices (page tabs, setting sections, a
// segment): the neighbour of current, stopping at both ends; values are
// compared as JSON so null works too. Something not in the row starts at
// the first choice.
function stepChoice(values, current, delta) {
  if (!values || values.length === 0) return current
  var key = JSON.stringify(current)
  var i = -1
  for (var j = 0; j < values.length; j++) if (JSON.stringify(values[j]) === key) { i = j; break }
  if (i < 0) return values[0]
  return values[Math.max(0, Math.min(values.length - 1, i + delta))]
}
