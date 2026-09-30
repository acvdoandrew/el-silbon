//! The words on screen, in English or in Spanish: one choice in Settings,
//! saved in this player's profile. Local only: it never goes on the wire
//! and is never hashed into the gameplay fingerprint, so two friends reading
//! different languages share one night (each reads the same Madrina chapter
//! in their own).
//!
//! Every pair of words the menus, the HUD, the hints, the captions and the
//! outcome card show lives in the tables below. Proper names (El Silbón,
//! Tureco, La Madrina, El Borracho / El Hijo / El Arriero, the places) are
//! the same in both. A sentence with numbers in it is written out in both
//! languages where it is formatted.

use serde::{Deserialize, Serialize};

/// The language the words on screen are in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// A new player's language.
    #[default]
    En,
    Es,
}

impl Lang {
    /// The English or the Spanish of two texts.
    pub fn pick<'a>(self, en: &'a str, es: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Es => es,
        }
    }

    /// A pair from the tables, in this language.
    pub fn say(self, words: Words) -> &'static str {
        self.pick(words.en, words.es)
    }

    /// The other language (the setting has two).
    pub fn toggled(self) -> Self {
        match self {
            Lang::En => Lang::Es,
            Lang::Es => Lang::En,
        }
    }

    /// The language's own name for itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
        }
    }
}

/// One text in both languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words {
    pub en: &'static str,
    pub es: &'static str,
}

const fn w(en: &'static str, es: &'static str) -> Words {
    Words { en, es }
}

/// The same in both (a name, a signature).
pub const fn both(text: &'static str) -> Words {
    Words { en: text, es: text }
}

/// The title screen, the pause card and every page under them.
pub mod menu {
    use super::{Words, w};

    /// Read in both languages, so it can always be found.
    pub const LANGUAGE: &str = "Language / Idioma";

    pub const TAGLINE: Words = w(
        "If you hear him close, he is far. If you hear him far, he is already here.",
        "Si lo oyes cerca, está lejos. Si lo oyes lejos, ya está aquí.",
    );

    pub const PLAY: Words = w("Play", "Jugar");
    pub const PLAY_FRIENDS: Words = w("Play with friends", "Jugar con amigos");
    pub const JOURNAL: Words = w("Journal", "Diario");
    pub const SETTINGS: Words = w("Settings", "Ajustes");
    pub const HOW_TO_PLAY: Words = w("How to play", "Cómo se juega");
    pub const CREDITS: Words = w("Credits", "Créditos");
    pub const QUIT: Words = w("Quit", "Salir");
    pub const BACK: Words = w("Back", "Volver");

    pub const GENTLE: Words = w("Gentle", "Suave");
    pub const NORMAL: Words = w("Normal", "Normal");
    pub const HARD: Words = w("Hard", "Difícil");
    pub const GENTLE_BLURB: Words = w(
        "For learning the llano: he notices you later, the torch lasts, the rhythm forgives.",
        "Para aprender el llano: te nota más tarde, la linterna dura más, el ritmo perdona.",
    );
    pub const NORMAL_BLURB: Words = w("The night as it was meant to be.", "La noche como debe ser.");
    pub const HARD_BLURB: Words = w(
        "He notices you farther off, the torch dies sooner, every rite angers him more.",
        "Te nota desde más lejos, la linterna se apaga antes, cada rito lo enfurece más.",
    );

    pub const SOLO_HEADING: Words = w("A night alone", "Una noche a solas");
    pub const SOLO_BODY: Words = w(
        "Leave the night blank for a new one, or type a night's number to play it again (friends can share numbers).",
        "Deja la noche en blanco para una nueva, o escribe el número de una noche para jugarla otra vez (los amigos pueden compartir números).",
    );
    pub const DIFFICULTY: Words = w("Difficulty", "Dificultad");
    pub const NIGHT: Words = w("Night", "Noche");
    pub const A_NEW_NIGHT_FIELD: Words = w("a new night", "una noche nueva");
    pub const BEGIN: Words = w("Begin the night", "Comenzar la noche");

    pub const FRIENDS_HEADING: Words = w("With friends", "Con amigos");
    pub const FRIENDS_BODY: Words = w(
        "Up to four players on the same local network, or far apart on a private VPN such as Tailscale. \
         One hosts the night; the others join with the host's address and receive the host's night and \
         difficulty.\nChoose who the others will see. \
         If a friend is already them, you are someone else (F7 in the lobby changes).",
        "Hasta cuatro jugadores en la misma red local, o lejos unos de otros en una VPN privada como Tailscale. \
         Uno hospeda la noche; los demás se unen con la dirección del anfitrión y reciben su noche y su \
         dificultad.\nElige a quién verán los demás. \
         Si un amigo ya lo es, serás otro (F7 en la sala de espera lo cambia).",
    );
    pub const WHO_YOU_ARE: Words = w("Who you are", "Quién eres");
    pub const HOST: Words = w("Host a night", "Hospedar una noche");
    pub const JOIN_FRIEND: Words = w("Join a friend", "Unirse a un amigo");

    pub const HOST_BODY: Words = w(
        "Give your friends this address. When everyone is in, press Enter in the night to begin.",
        "Dales esta dirección a tus amigos. Cuando estén todos, pulsa Enter en la noche para comenzar.",
    );
    pub const YOUR_ADDRESS: Words = w("Your address", "Tu dirección");
    pub const OPEN_SESSION: Words = w("Open the session", "Abrir la sesión");

    pub const JOIN_BODY: Words = w(
        "Type the address your host gave you (for example 192.168.1.20:5000). Their night and difficulty \
         come with the session.",
        "Escribe la dirección que te dio el anfitrión (por ejemplo 192.168.1.20:5000). Su noche y su \
         dificultad llegan con la sesión.",
    );
    pub const HOSTS_ADDRESS: Words = w("Host's address", "Dirección del anfitrión");
    pub const TYPE_IT_HERE: Words = w("type it here", "escríbela aquí");
    pub const JOIN: Words = w("Join", "Unirse");

    pub const NOT_FOUND: Words = w("· · ·  not yet found", "· · ·  aún sin encontrar");
    pub const NOT_TOLD: Words = w(
        "· · ·  the Madrina has not told it yet",
        "· · ·  la Madrina aún no lo ha contado",
    );

    pub const SAVED: Words = w("Saved as you change them.", "Se guardan al cambiarlos.");
    pub const VIDEO: Words = w("Video", "Video");
    pub const AUDIO: Words = w("Audio", "Audio");
    pub const CONTROLS: Words = w("Controls", "Controles");
    pub const CALIBRATE: Words = w("Calibrate brightness", "Calibrar el brillo");

    pub const DISPLAY_MODE: Words = w("Display mode", "Pantalla");
    pub const WINDOWED_LAUNCH: Words = w("Window (--windowed)", "Ventana (--windowed)");
    pub const FULLSCREEN: Words = w("Fullscreen", "Completa");
    pub const WINDOW: Words = w("Window", "Ventana");
    pub const FOV: Words = w("Field of view", "Campo de visión");
    pub const BRIGHTNESS: Words = w("Brightness", "Brillo");
    pub const CONTRAST: Words = w("Contrast", "Contraste");
    pub const HEAD_BOB: Words = w("Head bob", "Vaivén al caminar");
    pub const ON: Words = w("On", "Sí");
    pub const OFF: Words = w("Off", "No");

    pub const CALIBRATE_BODY: Words = w(
        "Raise Brightness until the middle hat is just barely visible. If the left one shows too, \
         raise Contrast until it is gone. The right one should be plain.",
        "Sube el Brillo hasta que el sombrero del medio apenas se vea. Si el de la izquierda también se ve, \
         sube el Contraste hasta que desaparezca. El de la derecha debe verse claro.",
    );
    pub const SHARED_BEHIND: Words = w(
        "\nThe shared night goes on behind this page.",
        "\nLa noche compartida sigue detrás de esta página.",
    );
    pub const DEFAULTS: Words = w("Defaults", "Restablecer");
    pub const DONE: Words = w("Done", "Listo");
    pub const HAT_VANISH: Words = w("should vanish", "debe desaparecer");
    pub const HAT_BARELY: Words = w("barely visible", "apenas se ve");
    pub const HAT_PLAIN: Words = w("plainly seen", "se ve claro");

    pub const AUDIO_BODY: Words = w(
        "Saved as you change them. His whistle follows the master volume alone.",
        "Se guardan al cambiarlos. Su silbido solo sigue al volumen general.",
    );
    pub const MASTER: Words = w("Master volume", "Volumen general");
    pub const MUSIC: Words = w("Music", "Música");
    pub const AMBIENCE: Words = w("Ambience", "Ambiente");
    pub const EFFECTS: Words = w("Effects", "Efectos");
    pub const CAPTIONS: Words = w("Whistle captions", "Subtítulos del silbido");

    pub const SENSITIVITY: Words = w("Mouse sensitivity", "Sensibilidad del mouse");
    pub const INVERT_Y: Words = w("Invert mouse Y", "Invertir el eje Y del mouse");

    pub const RESUME: Words = w("Resume", "Continuar");
    pub const RESTART: Words = w("Restart the night", "Reiniciar la noche");
    pub const LEAVE_TO_TITLE: Words = w("Leave to the title", "Volver al título");
    pub const END_FOR_ALL: Words = w("End the night for everyone", "Terminar la noche para todos");
    pub const PAUSED: Words = w("Paused", "En pausa");
    pub const PAUSED_SOLO: Words = w(
        "The night holds its breath. The mouse is free.",
        "La noche contiene el aliento. El mouse está libre.",
    );
    pub const PAUSED_SHARED: Words = w(
        "Only your controls stop: the shared night goes on.",
        "Solo se detienen tus controles: la noche compartida sigue.",
    );

    pub const PLAY_AGAIN: Words = w("Play the night again", "Jugar la noche otra vez");
    pub const NEW_NIGHT: Words = w("A new night", "Una noche nueva");
    pub const TITLE_MENU: Words = w("Title menu", "Menú principal");

    pub const LEAVE_HEADING: Words = w("Leave the night?", "¿Dejar la noche?");
    pub const LEAVE_SHARED: Words = w(
        "Everyone in your session goes back to their title screen.",
        "Todos en tu sesión vuelven a su pantalla de título.",
    );
    pub const LEAVE_SOLO: Words = w(
        "This night's progress is lost; pages you read stay in your journal.",
        "Se pierde lo avanzado esta noche; las páginas que leíste se quedan en tu diario.",
    );
    pub const STAY: Words = w("Stay", "Quedarse");
    pub const LEAVE: Words = w("Leave", "Irse");
    pub const QUIT_HEADING: Words = w("Quit the game?", "¿Salir del juego?");

    pub const HOW_TO: Words = w(
        "\
Lay the five bundles of the father's bones at the ceiba's roots, bring the power back at the windmill, \
open the padlocked key box (the house radio reads its three numbers out; the tag on the padlock names the \
station) and start the truck \
at the bridge. Survive its warm-up, everyone aboard. Or learn which of him walks tonight and name him \
at the ceiba (N) once every bone is home.\n\n\
The whistle lies: loud means he is far, thin and faint means he is near. Walls, trunks and tall grass \
break his sight; crouch to move quietly, running is heard far away. Your torch runs down, and its beam \
draws him. Lamplight and company calm fear; the dark and being alone feed it.\n\n\
While you lay bones, crank or turn the engine over, press Space as the needle crosses the marked zone. \
Ají stops him to count his bones. Tureco, tied behind the house, knows where he truly is. Caught with \
friends standing, you are carried off in his sack: pepper in his path drops you. With friends, whoever \
is aboard the ready truck can drive off without the others (X).\n\n\
WASD move · mouse look · Shift run · Ctrl/C crouch · E use (hold at sites) · F torch · Space rhythm · \
G drop a bundle · Q ají · V mark · M map · N name him · X drive off · Esc menu · F12 screenshot",
        "\
Pon los cinco atados de huesos del padre en las raíces de la ceiba, devuelve la luz en el molino, \
abre la caja de la llave con candado (la radio de la casa dice sus tres números; la etiqueta del candado \
nombra la emisora) y arranca la camioneta en el puente. Aguanta mientras calienta, todos a bordo. \
O descubre cuál de él camina esta noche y nómbralo en la ceiba (N) cuando todos los huesos estén en casa.\n\n\
El silbido miente: fuerte quiere decir que está lejos; fino y débil, que está cerca. Las paredes, los \
troncos y la paja alta le cortan la vista; agáchate para moverte en silencio, correr se oye lejos. Tu \
linterna se gasta, y su luz lo atrae. La luz de los faroles y la compañía calman el miedo; la oscuridad \
y la soledad lo alimentan.\n\n\
Mientras pones huesos, le das a la manivela o arrancas el motor, pulsa Espacio cuando la aguja cruce la \
zona marcada. El ají lo detiene a contar sus huesos. Tureco, amarrado detrás de la casa, sabe dónde está \
de verdad. Si te agarra con amigos en pie, te lleva en su saco: ají en su camino hace que te suelte. Con \
amigos, quien esté a bordo de la camioneta lista puede irse sin los demás (X).\n\n\
WASD moverse · mouse mirar · Shift correr · Ctrl/C agacharse · E usar (mantener en los sitios) · \
F linterna · Espacio ritmo · G soltar un atado · Q ají · V marcar · M mapa · N nombrarlo · X irse · \
Esc menú · F12 captura",
    );

    pub const CREDITS_TEXT: Words = w(
        "\
EL SILBÓN — The Return\n\n\
A game by Andrew Acevedo Mirena, made with Claude Code.\n\n\
Engine: Bevy 0.19 · Networking: Renet\n\
Fonts: Noto Sans and Noto Serif (SIL Open Font License)\n\
Every mesh, texture and sound is original, generated in code.\n\n\
The legend of El Silbón belongs to the llanos of Venezuela and Colombia. The Hacienda Santa Rosa, its \
people and every page in this game are fiction inspired by it.\n\n\
Thank you for playing.",
        "\
EL SILBÓN — The Return\n\n\
Un juego de Andrew Acevedo Mirena, hecho con Claude Code.\n\n\
Motor: Bevy 0.19 · Red: Renet\n\
Fuentes: Noto Sans y Noto Serif (SIL Open Font License)\n\
Cada malla, textura y sonido es original, generado en código.\n\n\
La leyenda de El Silbón pertenece a los llanos de Venezuela y Colombia. La Hacienda Santa Rosa, su \
gente y cada página de este juego son ficción inspirada en ella.\n\n\
Gracias por jugar.",
    );

    pub const ROLE_LLANERO: Words = w("Ranch hand", "Peón del hato");
    pub const ROLE_COPLERA: Words = w("Song keeper", "Cantadora de coplas");
    pub const ROLE_ENCARGADO: Words = w("Ranch caretaker", "Cuidador del hato");
    pub const ROLE_MUCHACHO: Words = w("Young local", "Muchacho del pueblo");
}

/// The HUD: objectives, the threat line, prompts, the hands at work, the
/// downed and the watching.
pub mod hud {
    use super::{Words, w};

    pub const GUIDE_LOBBY: Words = w(
        "Waiting for the host to begin (Enter).",
        "Esperando a que el anfitrión comience (Enter).",
    );
    pub const GUIDE_BONES: Words = w(
        "Bundles of bones lie in the landmarks — the ranch, the corral, the fields, the caño, the tower — and the ceiba is where they belong.",
        "Hay atados de huesos por todo el hato — el rancho, el corral, los sembradíos, el caño, la torre — y su lugar es la ceiba.",
    );
    pub const GUIDE_POWER: Words = w(
        "The bones are home. Now the windmill: hold E at the pump to bring the lights back. It is loud. \
         Or, if you know which of him walks tonight, name him at the ceiba (N).",
        "Los huesos están en casa. Ahora el molino: mantén E en la bomba para traer la luz. Hace ruido. \
         O, si sabes cuál de él camina esta noche, nómbralo en la ceiba (N).",
    );
    pub const GUIDE_KEY: Words = w(
        "The truck key is padlocked in a box on the crates by the windmill. A tag on it names a frequency: the shelf radio in the house reads the numbers out, in pips.",
        "La llave de la camioneta está en una caja con candado, sobre las cajas junto al molino. Una etiqueta nombra una frecuencia: la radio del estante, en la casa, dice los números en pitidos.",
    );
    pub const GUIDE_IGNITION: Words = w(
        "Hold E at the truck's ignition. The engine will roar, and he will come.",
        "Mantén E en el encendido de la camioneta. El motor rugirá, y él vendrá.",
    );
    pub const GUIDE_WARM: Words = w(
        "Survive. Keep watch while the engine warms.",
        "Aguanta. Vigila mientras calienta el motor.",
    );
    pub const GUIDE_ABOARD_SHARED: Words = w(
        "Everyone standing: get into the truck zone! Or, aboard, press X to drive off without the others.",
        "Todos los que sigan en pie: ¡a la camioneta! O, ya a bordo, pulsa X para irte sin los demás.",
    );
    pub const GUIDE_ABOARD: Words = w(
        "Everyone standing: get into the truck zone!",
        "Todos los que sigan en pie: ¡a la camioneta!",
    );

    pub const TRUCK_WARM: Words = w(
        "The truck is warm — everyone aboard!",
        "La camioneta está caliente — ¡todos a bordo!",
    );
    pub const OPEN_KEY_BOX: Words = w(
        "Open the key box at the windmill (three numbers, read out by the house radio)",
        "Abre la caja de la llave en el molino (tres números, los dice la radio de la casa)",
    );
    pub const START_TRUCK: Words = w("Start the truck at the bridge", "Arranca la camioneta en el puente");
    pub const RESTORE_POWER: Words = w("Restore power at the windmill", "Devuelve la luz en el molino");
    /// The lamp lines, as the power line names them.
    pub const LINES: [Words; 3] = [
        w("hacienda", "la hacienda"),
        w("corral", "el corral"),
        w("bridge", "el puente"),
    ];
    pub const AND: Words = w(" and ", " y ");

    pub const SEEN_YOU: Words = w(
        "He has seen you — break his line of sight",
        "Te vio — rompe su línea de vista",
    );
    pub const COMING: Words = w(
        "He is coming — get behind solid walls!",
        "Ya viene — ¡ponte tras paredes sólidas!",
    );
    pub const OUT_OF_SIGHT: Words = w("Out of his sight… stay hidden", "Fuera de su vista… no te dejes ver");
    pub const COUNTING_SLIP: Words = w(
        "He kneels to count his bones — slip away",
        "Se arrodilla a contar sus huesos — escápate",
    );
    pub const COUNTING: Words = w("He kneels to count his bones", "Se arrodilla a contar sus huesos");
    pub const FROZEN: Words = w("Frozen with fright…", "Paralizado del susto…");
    pub const CATTLE: Words = w(
        "The cattle are bellowing — the whole llano can hear",
        "El ganado está bramando — todo el llano lo oye",
    );
    pub const ENGINE: Words = w(
        "The engine roars — it carries for miles",
        "El motor ruge — se oye a leguas",
    );
    pub const BEACON: Words = w(
        "The beacon burns — he is drawn to the light",
        "El farol de la torre arde — la luz lo atrae",
    );

    pub const YOU_SUFFIX: Words = w(" (you)", " (tú)");
    pub const TORCH_DEAD: Words = w("Torch: dead", "Linterna: agotada");

    pub const HOLD: [Words; 7] = [
        w("Laying the bones down…", "Poniendo los huesos…"),
        w("Praying at the roots…", "Rezando en las raíces…"),
        w("Cranking the pump…", "Dándole a la manivela…"),
        w("Turning the key…", "Girando la llave…"),
        w("Lighting the beacon…", "Encendiendo el farol…"),
        w("Helping them up…", "Ayudando a levantarse…"),
        w("Untying Tureco…", "Soltando a Tureco…"),
    ];
    pub const STALL_BONES: Words = w("The bones slip from your hands…", "Los huesos se te resbalan…");
    pub const STALL_CRANK: Words = w("The crank kicks back…", "La manivela te rebota…");
    pub const STALL_ENGINE: Words = w("The engine floods…", "El motor se ahoga…");

    pub const TAKE_BONES: Words = w(
        "[E] Take the bones — heavy, and they rattle",
        "[E] Tomar los huesos — pesan, y suenan",
    );
    pub const TAKE_PEPPERS: Words = w("[E] Take the peppers", "[E] Tomar el ají");
    pub const TAKE_BATTERIES: Words = w("[E] Take the spare batteries", "[E] Tomar las pilas de repuesto");
    pub const PUT_NOTE_DOWN: Words = w("[E] Put the note down", "[E] Dejar la nota");
    pub const READ_NOTE: Words = w("[E] Read the note", "[E] Leer la nota");
    pub const LAY_BONES: Words = w("[Hold E] Lay the bones down", "[Mantén E] Poner los huesos");
    pub const PRAY_OR_NAME: Words = w(
        "[Hold E] Pray   ·   [N] Name which of him walks tonight",
        "[Mantén E] Rezar   ·   [N] Nombrar cuál de él camina esta noche",
    );
    pub const PRAY: Words = w(
        "[Hold E] Pray at the roots — it steadies you, but the ceiba hears",
        "[Mantén E] Rezar en las raíces — te calma, pero la ceiba oye",
    );
    pub const CRANK: Words = w(
        "[Hold E] Crank the pump — loud!",
        "[Mantén E] Darle a la bomba — ¡hace ruido!",
    );
    pub const NEED_BONES: Words = w(
        "The engine will not turn while the bones are unrested",
        "El motor no arranca mientras los huesos no descansen",
    );
    pub const NEED_POWER: Words = w("Nothing turns over — the power is out", "Nada arranca — no hay luz");
    pub const NEED_KEY: Words = w(
        "No key in the ignition — it is padlocked in the box at the windmill",
        "No hay llave en el encendido — está bajo candado en la caja del molino",
    );
    pub const START: Words = w(
        "[Hold E] Start the truck — the roar will carry",
        "[Mantén E] Arrancar la camioneta — el rugido se oirá lejos",
    );
    pub const LIGHT_BEACON: Words = w(
        "[Hold E] Light the beacon — he will come to the light",
        "[Mantén E] Encender el farol — vendrá a la luz",
    );
    pub const KEY_BOX: Words = w(
        "[E] The key box — a three-number padlock",
        "[E] La caja de la llave — un candado de tres números",
    );
    pub const UNTIE_DOG: Words = w(
        "[Hold E] Untie Tureco — he fears nothing, and HE fears dogs",
        "[Mantén E] Soltar a Tureco — no le teme a nada, y ÉL les teme a los perros",
    );
    pub const PANEL: Words = w(
        "[E] Switch the lamp lines — the old dynamo carries only two",
        "[E] Cambiar las líneas de faroles — el viejo dínamo solo aguanta dos",
    );
    pub const HELP_UP: Words = w("[Hold E] Help them up", "[Mantén E] Ayudar a levantarse");
    pub const RADIO: Words = w(
        "[E] Turn the radio's dial — it squeals",
        "[E] Girar el dial de la radio — chilla",
    );
    pub const BONES_CLOSER: Words = w("Bones — move closer", "Huesos — acércate");
    pub const PEPPERS_CLOSER: Words = w("Peppers — move closer", "Ají — acércate");
    pub const BATTERIES_CLOSER: Words = w("Batteries — move closer", "Pilas — acércate");
    pub const MOVE_CLOSER: Words = w("move closer", "acércate");
    pub const RADIO_OFF: Words = w("off", "apagada");

    pub const DOWN_HELP: Words = w(
        "Crawl (WASD) toward your friends   ·   V  cry for help (he may hear it too)   ·   F  your torch shows where you lie",
        "Arrástrate (WASD) hacia tus amigos   ·   V  pide auxilio (él también puede oírlo)   ·   F  tu linterna muestra dónde estás",
    );
    pub const IN_SACK: Words = w(
        "IN HIS SACK\nThe bones press on you in the dark. Only ají in his path will make him drop you.",
        "EN SU SACO\nLos huesos te aprietan en la oscuridad. Solo el ají en su camino hará que te suelte.",
    );
    pub const DIED_SHARED: Words = w("YOU DIED\nWatch over your friends", "MORISTE\nCuida a tus amigos");
    pub const DIED: Words = w("YOU DIED", "MORISTE");

    /// The controls reminder, and while watching a friend only the switch keys.
    pub const CONTROLS: Words = w(
        "WASD move · Shift run · Ctrl crouch · E use / hold · F light · G drop · Q ají · V mark (down: cry for help) · M map · Esc",
        "WASD moverse · Shift correr · Ctrl agacharse · E usar / mantener · F luz · G soltar · Q ají · V marcar (caído: pedir auxilio) · M mapa · Esc",
    );
    pub const CONTROLS_WATCHING: Words = w(
        "A / D or ← / →  watch another friend · M map · Esc",
        "A / D o ← / →  mirar a otro amigo · M mapa · Esc",
    );

    pub const NAMES: [Words; 3] = [
        w(
            "1  El Borracho — the drunkard's return",
            "1  El Borracho — el que vuelve borracho",
        ),
        w("2  El Hijo — the son himself", "2  El Hijo — el hijo mismo"),
        w("3  El Arriero — the drover", "3  El Arriero — el que arrea el ganado"),
    ];

    /// The whistle's captions: the impression, loud (so far), middling,
    /// faint (so near); and one fear made up.
    pub const CAPTION_LOUD: Words = w(
        "A whistle — loud, as if right beside you.",
        "Un silbido — fuerte, como si estuviera a tu lado.",
    );
    pub const CAPTION_MIDDLING: Words = w(
        "A whistle — somewhere out across the grass.",
        "Un silbido — en algún lugar, más allá de la paja.",
    );
    pub const CAPTION_FAINT: Words = w(
        "A whistle — thin and faint, far, far away…",
        "Un silbido — fino y débil, lejos, muy lejos…",
    );
    pub const CAPTION_PHANTOM: Words = w(
        "A whistle…? Or only the blood in your ears.",
        "¿Un silbido…? O solo la sangre en tus oídos.",
    );
}

/// The teaching hints over the night.
pub mod hint {
    use super::{Words, w};

    pub const RELIC_TAKEN: Words = w(
        "The bundle is heavy, and it rattles. Something out on the llano knows.",
        "El atado pesa, y suena. Algo allá en el llano lo sabe.",
    );
    pub const RELIC_DROPPED: Words = w(
        "You set the bones down. Gently.",
        "Dejas los huesos en el suelo. Con cuidado.",
    );
    pub const SKILL: Words = w(
        "Keep the rhythm: press Space as the needle crosses the marked zone. Miss, and it screeches and the work slips back.",
        "Lleva el ritmo: pulsa Espacio cuando la aguja cruce la zona marcada. Si fallas, chirría y el trabajo retrocede.",
    );
    pub const SILENCE: Words = w("Even the frogs have stopped.", "Hasta los sapos se callaron.");
    pub const WEEPING: Words = w(
        "Across the llano, a grown man is weeping.",
        "Al otro lado del llano, un hombre hecho y derecho está llorando.",
    );
    pub const DOG_FREED: Words = w(
        "Tureco shakes himself and falls in at your heels.",
        "Tureco se sacude y se te pega a los talones.",
    );
    pub const DOG_GROWL: Words = w(
        "Tureco growls low at the dark. He is near — whatever the whistle says.",
        "Tureco le gruñe bajito a la oscuridad. Él está cerca — diga lo que diga el silbido.",
    );
    pub const DOG_BARK: Words = w(
        "Tureco barks — and out in the dark, something flinches away.",
        "Tureco ladra — y allá en lo oscuro, algo se aparta de un respingo.",
    );
    pub const HAULED: Words = w(
        "He stuffed them into his sack and walks off! Get ají in his path before he is gone.",
        "¡Lo metió en su saco y se lo lleva! Pon ají en su camino antes de que desaparezca.",
    );
    pub const SACK_FALLS: Words = w("The sack falls!", "¡Se cae el saco!");
    pub const SACK_FALLS_HELP: Words = w(
        "The sack falls! Follow their groans, their torch and the red mark — then hold E beside them.",
        "¡Se cae el saco! Sigue sus quejidos, su linterna y la marca roja — y mantén E a su lado.",
    );
    pub const TAKEN: Words = w(
        "He is gone into the grass. And so are they.",
        "Se perdió en la paja. Y se los llevó.",
    );
    pub const WHIP: Words = w(
        "A whip cracks somewhere out in the dark.",
        "Restalla un látigo en algún lugar de la oscuridad.",
    );
    pub const BOTTLES: Words = w(
        "Glass knocks against glass, out in the grass.",
        "Un vidrio choca con otro, allá en la paja.",
    );
    pub const NAME_WRONG: Words = w(
        "Wrong name. The ceiba shudders — and he comes, furious.",
        "Nombre equivocado. La ceiba se estremece — y él viene, furioso.",
    );
    pub const KEY_FOUND: Words = w(
        "The padlock gives. The truck key is ours.",
        "El candado cede. La llave de la camioneta es nuestra.",
    );
    pub const LINES_SWITCHED: Words = w(
        "The dynamo groans as the lines change. Somewhere, lamps die; somewhere else, they wake.",
        "El dínamo gime al cambiar las líneas. En un lado se apagan faroles; en otro, despiertan.",
    );
    pub const LOCK_RATTLE: Words = w(
        "Wrong numbers. The padlock rattles, loud in the quiet.",
        "Números equivocados. El candado traquetea, fuerte en el silencio.",
    );
    pub const DRAG: Words = w(
        "Something heavy was dragged through the mud here. Recently.",
        "Aquí arrastraron algo pesado por el barro. Hace poco.",
    );
    pub const MISSED: Words = w(
        "It screeches across the llano. He heard that.",
        "El chirrido cruza el llano. Él lo oyó.",
    );
    pub const BATTERIES: Words = w(
        "Spare batteries. The beam steadies — and it is the brightest thing on the llano.",
        "Pilas de repuesto. La luz se afirma — y es lo más brillante del llano.",
    );
    pub const ALL_HOME: Words = w(
        "The bones are home. Bring the power back, then start the truck.",
        "Los huesos están en casa. Devuelve la luz y luego arranca la camioneta.",
    );
    pub const POWER: Words = w(
        "The lamps hum back to life. Light steadies the nerves — stay near it.",
        "Los faroles vuelven a zumbar. La luz calma los nervios — quédate cerca.",
    );
    pub const TRUCK: Words = w(
        "The engine roars — he heard it. Hold out until it warms up.",
        "El motor ruge — él lo oyó. Aguanta hasta que caliente.",
    );
    pub const CROUCH: Words = w(
        "Something moved out on the llano. Crouch (Ctrl) to move quietly; running is loud.",
        "Algo se movió allá en el llano. Agáchate (Ctrl) para moverte en silencio; correr hace ruido.",
    );
    pub const FRIEND_SEEN: Words = w("He has seen your friend.", "Vio a tu amigo.");
    pub const FRIEND_HUNTED: Words = w("He is coming for your friend!", "¡Viene por tu amigo!");
    pub const FRIEND_SUSTO: Words = w(
        "Susto — fright freezes your friend.",
        "Susto — el miedo paraliza a tu amigo.",
    );
    pub const SEEN: Words = w(
        "He has seen you. Get solid walls between you before he comes.",
        "Te vio. Pon paredes sólidas de por medio antes de que venga.",
    );
    pub const HUNT: Words = w(
        "He is coming. Break his line of sight!",
        "Ya viene. ¡Rompe su línea de vista!",
    );
    pub const AVERTED: Words = w("He lost sight of you.", "Te perdió de vista.");
    pub const LOST_TRACK: Words = w(
        "He lost your trail and sinks into the grass. He will rise somewhere else.",
        "Perdió tu rastro y se hunde en la paja. Saldrá por otro lado.",
    );
    pub const HAS_YOU: Words = w("He has you.", "Te agarró.");
    pub const YOU_DOWN: Words = w(
        "You are down. Hold on — someone may reach you.",
        "Estás caído. Aguanta — alguien puede llegar.",
    );
    pub const FRIEND_DOWN_WATCHING: Words = w("A friend is down.", "Cayó un amigo.");
    pub const FRIEND_DOWN: Words = w(
        "A friend is down! Hold E beside them to help them up.",
        "¡Cayó un amigo! Mantén E a su lado para levantarlo.",
    );
    pub const REVIVED: Words = w("Back on your feet. Keep moving.", "De pie otra vez. Sigue moviéndote.");
    pub const DIED: Words = w("Someone did not make it.", "Alguien no lo logró.");
    pub const AJI: Words = w(
        "Hot peppers. Press Q to scatter them — he stops to count his bones.",
        "Ají picante. Pulsa Q para regarlo — se detiene a contar sus huesos.",
    );
    pub const COUNTING: Words = w(
        "He kneels to count his bones. Slip away, quietly.",
        "Se arrodilla a contar sus huesos. Escápate, en silencio.",
    );
    pub const COUNTED: Words = w("He has finished counting.", "Terminó de contar.");
    pub const SUSTO: Words = w(
        "Susto — fright freezes you. Stay in the light and close to your friends.",
        "Susto — el miedo te paraliza. Quédate en la luz y cerca de tus amigos.",
    );
    pub const CATTLE: Words = w(
        "The cattle bellow. Everything heard that.",
        "El ganado brama. Todo el llano lo oyó.",
    );
    pub const BEACON: Words = w(
        "The beacon flares. He turns toward the light.",
        "El farol se enciende. Él se vuelve hacia la luz.",
    );
    pub const PRAYED: Words = w("The fear eases.", "El miedo se calma.");
    pub const DAWN: Words = w(
        "A rooster crows. Dawn: he sinks into the grass.",
        "Canta un gallo. Amanece: se hunde en la paja.",
    );
    pub const CALL: Words = w(
        "Help! A friend cries out from the grass — follow the voice and the red mark.",
        "¡Auxilio! Un amigo grita desde la paja — sigue la voz y la marca roja.",
    );
    pub const LOUD_LESSON: Words = w(
        "A loud whistle, as if right beside you. The old rule: when he sounds near, he is far.",
        "Un silbido fuerte, como si estuviera a tu lado. La regla de los viejos: cuando suena cerca, está lejos.",
    );
    pub const FAINT_LESSON: Words = w(
        "A thin whistle, far, far away… so he is NEAR. Get solid walls between you.",
        "Un silbido fino, lejos, muy lejos… así que está CERCA. Pon paredes sólidas de por medio.",
    );
}

/// Panels spawned once: the briefing, the padlock, the naming, the map.
pub mod panel {
    use super::{Words, w};

    pub const BRIEFING_SUBTITLE: Words = w(
        "The Return — some whistles should never be followed",
        "The Return — hay silbidos que nunca se deben seguir",
    );
    pub const BRIEFING_STORY: Words = w(
        "Los Llanos, 1998. Your truck died at the river bridge and the road home is thirty \
         kilometres of dark. Ahead: a hacienda with a lamp still burning, a windmill that \
         could bring the power back, and five bundles of bones taken from El Silbón's sack. \
         He is out in the rain, he wants them back, and he listens to everything.",
        "Los Llanos, 1998. Tu camioneta murió en el puente del río y el camino a casa son treinta \
         kilómetros de oscuridad. Adelante: una hacienda con un farol todavía encendido, un molino que \
         podría devolver la luz y cinco atados de huesos sacados del saco de El Silbón. \
         Él anda afuera bajo la lluvia, los quiere de vuelta, y lo escucha todo.",
    );
    pub const BRIEFING_RULES: Words = w(
        "The whistle lies: loud means he is far, thin means he is near. Walls, trunks and tall \
         grass break his sight. Everything you do makes a sound — crouch to sneak, run to be \
         heard; rain and thunder hide your steps. Fear grows in the dark and alone.",
        "El silbido miente: fuerte quiere decir que está lejos; fino, que está cerca. Las paredes, los \
         troncos y la paja alta le cortan la vista. Todo lo que haces suena — agáchate para escabullirte, \
         corre y te oirán; la lluvia y los truenos tapan tus pasos. El miedo crece en la oscuridad y a solas.",
    );
    pub const BRIEFING_GOALS: Words = w(
        "Find the five bundles and lay them at the ceiba · restore power at the windmill · open \
         the padlocked key box (the house radio reads its numbers out) · start the truck and survive its \
         roar. Or learn which of him walks tonight and name him at the ceiba. Keep the rhythm \
         of the work (Space). Your torch runs down and its beam draws him. Ají stops him for a \
         while; Tureco, if you untie him, knows where he is. Get the fallen out of his sack.",
        "Encuentra los cinco atados y ponlos en la ceiba · devuelve la luz en el molino · abre la caja \
         de la llave con candado (la radio de la casa dice sus números) · arranca la camioneta y aguanta su \
         rugido. O descubre cuál de él camina esta noche y nómbralo en la ceiba. Lleva el ritmo del \
         trabajo (Espacio). Tu linterna se gasta y su luz lo atrae. El ají lo detiene un rato; Tureco, \
         si lo sueltas, sabe dónde está. Saca a los caídos de su saco.",
    );
    pub const BRIEFING_KEYS: Words = w(
        "WASD move · Mouse look · Shift run · Ctrl/C crouch · E or click use (hold at sites) · F flashlight\n\
         Space skill check · G put a bundle down · Q scatter ají · V mark a spot (down: cry for help) · N name him (at the ceiba) · X drive off (with friends)\n\
         M map · Esc pause · F12 screenshot",
        "WASD moverse · Mouse mirar · Shift correr · Ctrl/C agacharse · E o clic usar (mantener en los sitios) · F linterna\n\
         Espacio prueba de ritmo · G soltar un atado · Q regar ají · V marcar un sitio (caído: pedir auxilio) · N nombrarlo (en la ceiba) · X irse (con amigos)\n\
         M mapa · Esc pausa · F12 captura",
    );
    pub const BEGIN: Words = w(
        "Begin — click to capture the mouse",
        "Comenzar — haz clic para capturar el mouse",
    );
    pub const BACK_TO_TITLE: Words = w("Back to the title", "Volver al título");

    pub const PADLOCK: Words = w("PADLOCK", "CANDADO");
    pub const PADLOCK_HELP: Words = w(
        "1 · 2 · 3 turn the dials (Shift back)   ·   Enter tries   ·   E closes\nA wrong try rattles — and the llano hears.",
        "1 · 2 · 3 giran los discos (Shift atrás)   ·   Enter prueba   ·   E cierra\nUn intento fallido traquetea — y el llano lo oye.",
    );
    pub const WHICH_OF_HIM: Words = w("WHICH OF HIM WALKS TONIGHT?", "¿CUÁL DE ÉL CAMINA ESTA NOCHE?");
    pub const NAMING_HELP: Words = w(
        "1 · 2 · 3 choose   ·   Enter names him   ·   N closes\nA wrong name enrages him, and the ceiba will not listen for a while.",
        "1 · 2 · 3 elegir   ·   Enter lo nombra   ·   N cierra\nUn nombre equivocado lo enfurece, y la ceiba no escuchará por un rato.",
    );
    pub const PUT_PAGE_DOWN: Words = w("[E] put the page down", "[E] dejar la página");
    pub const MAP_LEGEND: Words = w(
        "N is up · you are the bright marker · V marks a spot for everyone · M closes",
        "El norte arriba · tú eres la marca brillante · V marca un sitio para todos · M cierra",
    );
}

/// The card after a night: its title, its tale, the marks and the awards.
pub mod outcome {
    use super::{Words, w};

    pub const RESTED_TITLE: Words = w("He is laid to rest.", "Ya descansa.");
    pub const RESTED: Words = w(
        "You named him at the roots of the ceiba, with his father's bones all home. The whistle unwinds, \
         lower and lower, into the rain, and the llano is only the llano again.",
        "Lo nombraste en las raíces de la ceiba, con todos los huesos de su padre en casa. El silbido se \
         deshace, cada vez más bajo, en la lluvia, y el llano vuelve a ser solo el llano.",
    );
    pub const LEFT_YOU_TITLE: Words = w("They left without you.", "Se fueron sin ti.");
    pub const LEFT_YOU: Words = w(
        "The truck's lights shrink down the road and the engine fades into the rain. The llano is very \
         quiet. Somewhere a whistle starts, thin and far away…",
        "Las luces de la camioneta se achican camino abajo y el motor se pierde en la lluvia. El llano \
         está muy callado. En algún lugar empieza un silbido, fino y lejano…",
    );
    pub const AWAY_TITLE: Words = w("The truck pulls away.", "La camioneta se aleja.");
    pub const LEFT_THEM: Words = w(
        "You did not wait. In the mirror the llano closes over the ones you left, and somewhere out there \
         a whistle goes thin and far away…",
        "No esperaste. En el retrovisor el llano se cierra sobre los que dejaste, y allá afuera un \
         silbido se vuelve fino y lejano…",
    );
    pub const DAWN_TITLE: Words = w("You lived, but he will be back.", "Sobreviviste, pero él volverá.");
    pub const AWAY: Words = w(
        "Behind you the rain hushes the llano. The bones rest in the ceiba's roots, and somewhere out there \
         a whistle goes thin and far away… for now.",
        "Detrás de ti la lluvia calla el llano. Los huesos descansan en las raíces de la ceiba, y allá \
         afuera un silbido se vuelve fino y lejano… por ahora.",
    );
    pub const FOUND_TITLE: Words = w("He found you.", "Te encontró.");
    pub const FOUND: Words = w(
        "The whistle had gone thin and far away — he was already near.\n\
         Next time: stay together, stay in the light, and put walls between you when it fades.",
        "El silbido se había vuelto fino y lejano — ya estaba cerca.\n\
         La próxima vez: quédense juntos, quédense en la luz, y pongan paredes de por medio cuando se apague.",
    );
    pub const WHOLE_TALE: Words = w(
        "The whole tale is told: the son, the deer, the father, the grandfather's curse, \
         the torn sack and the ranch that tried to lay the father down. \
         Somewhere the radio still says: if you hear the whistle, remember…",
        "El cuento está completo: el hijo, el venado, el padre, la maldición del abuelo, \
         el saco roto y el hato que quiso darle descanso al padre. \
         En algún lugar la radio todavía dice: si oyes el silbido, acuérdate…",
    );
    pub const BORRACHO: Words = w(
        "the drunkard's return (El Borracho)",
        "el que vuelve borracho (El Borracho)",
    );
    pub const HIJO: Words = w("the son himself (El Hijo)", "el hijo mismo (El Hijo)");
    pub const ARRIERO: Words = w("the drover (El Arriero)", "el que arrea el ganado (El Arriero)");

    pub const SILENT: Words = w(
        "Silent as the grass (he never saw you)",
        "Callado como la paja (nunca te vio)",
    );
    pub const UNBROKEN: Words = w("Unbroken (nobody fell)", "Enteros (nadie cayó)");
    pub const NOBODY_LEFT: Words = w(
        "Nobody left behind (the fallen got up)",
        "Nadie se quedó atrás (los caídos se levantaron)",
    );
    pub const NAMED_HIM: Words = w("The one who named him", "Quien lo nombró");
    pub const QUICK: Words = w(
        "Quick hands (out before eight minutes)",
        "Manos rápidas (fuera antes de ocho minutos)",
    );
    pub const KEEPER: Words = w(
        "Keeper of the tale (every page read)",
        "Guardián del cuento (todas las páginas leídas)",
    );
    pub const EACH_ALONE: Words = w(
        "Every one for themselves (drove off early)",
        "Cada quien por su lado (se fueron antes)",
    );

    pub const LEFT_BEHIND: Words = w("Left behind on the llano", "Abandonado en el llano");
    pub const SACK_RIDER: Words = w("Rode in his sack", "Viajó en su saco");
    pub const FIRST_TO_FALL: Words = w("First to fall", "El primero en caer");
    pub const SCREAMER: Words = w("Screamed the most", "El que más gritó");
    pub const BUTTERFINGERS: Words = w(
        "Butterfingers (dropped the bones)",
        "Manos de mantequilla (se le cayeron los huesos)",
    );
    pub const FAVOURITE: Words = w("His favourite (warned the most)", "Su favorito (lo vio más veces)");
    pub const GUARDIAN: Words = w(
        "Guardian angel (got someone up)",
        "Ángel de la guarda (levantó a alguien)",
    );
    pub const BEARER: Words = w("Bone bearer (laid the most)", "Cargador de huesos (puso más que nadie)");
    pub const PEPPER_HAND: Words = w("Ají in every pocket", "Ají en cada bolsillo");
    pub const STAMPEDE: Words = w("Started a stampede", "Armó una estampida");
    pub const TURECOS_FRIEND: Words = w("Tureco's friend", "Amigo de Tureco");
    pub const UNTOUCHED: Words = w(
        "Untouched (never warned, never scared)",
        "Intocable (nunca lo vio, nunca se asustó)",
    );
    pub const YOU: Words = w("You", "Tú");
    pub const AWARDS: Words = w("Tonight's awards", "Los honores de esta noche");
}

/// The shared session's banner and the fallen's watching line.
pub mod net {
    use super::{Words, w};

    pub const LOBBY: Words = w(
        "Lobby: the host presses Enter when everyone is connected | F7: be someone else",
        "Sala de espera: el anfitrión pulsa Enter cuando todos estén conectados | F7: ser otro",
    );
    pub const VICTORY: Words = w(
        "SHARED VICTORY: the truck is away with everyone still standing.",
        "VICTORIA COMPARTIDA: la camioneta se fue con todos todavía en pie.",
    );
    pub const FAILURE: Words = w(
        "SHARED FAILURE: nobody is left on their feet. The host can restart.",
        "DERROTA COMPARTIDA: nadie quedó en pie. El anfitrión puede reiniciar.",
    );
    pub const DAWN: Words = w(
        "SHARED DAWN: the rooster crowed with bones still out. He will be back. The host can restart.",
        "ALBA COMPARTIDA: el gallo cantó con huesos todavía afuera. Él volverá. El anfitrión puede reiniciar.",
    );
    pub const SACK: Words = w(
        "IN HIS SACK: ají in his path, or Tureco's bark, makes him drop you.",
        "EN SU SACO: el ají en su camino, o el ladrido de Tureco, hace que te suelte.",
    );
    pub const DOWN: Words = w(
        "DOWN: crawl toward your friends | V: call for help (he may hear it too).",
        "CAÍDO: arrástrate hacia tus amigos | V: pide auxilio (él también puede oírlo).",
    );
    pub const GONE: Words = w("You are gone for the night.", "Esta noche ya no estás.");
    pub const KEYS: Words = w(
        "V: mark | Q: pepper | G: put a bundle down | Esc: local menu (world continues)",
        "V: marcar | Q: ají | G: soltar un atado | Esc: menú local (el mundo sigue)",
    );
    pub const END_SESSION: Words = w("end session", "terminar la sesión");
    pub const LEAVE_SESSION: Words = w("leave session", "salir de la sesión");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_or_older_profile_reads_english_and_the_choice_flips_both_ways() {
        assert_eq!(Lang::default(), Lang::En);
        // Saved as a word, so the profile stays readable.
        let saved = serde_json::to_string(&Lang::Es).expect("serializes");
        assert_eq!(serde_json::from_str::<Lang>(&saved).ok(), Some(Lang::Es));
        for l in [Lang::En, Lang::Es] {
            assert_ne!(l.toggled(), l);
            assert_eq!(l.toggled().toggled(), l);
        }
    }

    #[test]
    fn each_language_shows_only_its_own_half() {
        let pair = w("left", "right");
        assert_eq!(Lang::En.say(pair), "left");
        assert_eq!(Lang::Es.say(pair), "right");
        let same = both("— M.");
        assert_eq!(Lang::En.say(same), Lang::Es.say(same));
    }
}
