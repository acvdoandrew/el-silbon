//! The pages left around the hacienda: fiction inspired by the legend of El
//! Silbón, told in what people of the llanos kept and wrote on in the early
//! 1960s — a foreman's ledger, a letter, a copla on the back of a sack tag,
//! the parish register, a telegram, an almanac's margins, a newspaper
//! clipping, a prayer card, the back of a photograph, the radio. Read in any
//! order they tell one story: the son who killed his father for a deer's
//! entrails, the grandfather's curse, the sack of bones; and the night the
//! sack tore on the caño fence and the Hacienda Santa Rosa tried to lay the
//! father to rest. Many also teach a rule of the night. Every page is
//! written in Spanish and in English; a player reads the one their Settings
//! choose (`lang`).

use crate::lang::{Lang, Words, both};

/// What a page is: how it looks and how it is introduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medium {
    Note,
    Letter,
    Ledger,
    Clipping,
    Photograph,
    Almanac,
    Copla,
    Telegram,
    Register,
    Logbook,
    PrayerCard,
    Radio,
}

impl Medium {
    /// The kind of page, as the header names it.
    pub fn label(self, lang: Lang) -> &'static str {
        let (es, en) = match self {
            Medium::Note => ("Nota", "Note"),
            Medium::Letter => ("Carta", "Letter"),
            Medium::Ledger => ("Cuaderno del hato", "Ranch ledger"),
            Medium::Clipping => ("Recorte de periódico", "Newspaper clipping"),
            Medium::Photograph => ("Al dorso de una fotografía", "On the back of a photograph"),
            Medium::Almanac => ("Almanaque, al margen", "Almanac, in the margin"),
            Medium::Copla => ("Copla", "Sung verses"),
            Medium::Telegram => ("Telegrama", "Telegram"),
            Medium::Register => ("Libro parroquial", "Parish register"),
            Medium::Logbook => ("Libro de guardia", "Watch log"),
            Medium::PrayerCard => ("Estampita", "Prayer card"),
            // A station's name and hour: the same in both.
            Medium::Radio => ("Radio · La Voz del Llano, 11 p.m.", "Radio · La Voz del Llano, 11 p.m."),
        };
        lang.pick(en, es)
    }

    /// Paper colour of the page (sRGB).
    pub fn paper(self) -> [f32; 3] {
        match self {
            Medium::Photograph => [0.62, 0.55, 0.44],
            Medium::Telegram => [0.84, 0.82, 0.68],
            Medium::Clipping => [0.8, 0.77, 0.68],
            Medium::PrayerCard => [0.86, 0.82, 0.72],
            Medium::Register => [0.7, 0.62, 0.48],
            Medium::Radio => [0.2, 0.18, 0.15],
            _ => [0.78, 0.72, 0.58],
        }
    }
}

/// One page, in both languages.
pub struct Page {
    pub medium: Medium,
    pub title: Words,
    pub es: &'static str,
    pub en: &'static str,
    /// Who wrote it, or where it was found.
    pub by: Words,
}

impl Page {
    /// The page's words, in one language.
    pub fn text(&self, lang: Lang) -> &'static str {
        lang.pick(self.en, self.es)
    }
}

/// How many pages the hacienda holds (ids `0..PAGES`).
pub const PAGES: u8 = 20;

/// The page for a note id from the layout (0 is the one on the table).
pub fn note(id: u8) -> Page {
    use Medium::*;
    match id {
        0 => Page {
            medium: Note,
            title: Words {
                es: "Sobre la mesa",
                en: "On the table",
            },
            es: "Si lo oyes cerca, está lejos.\n\
                 Si lo oyes lejos, ya está aquí.\n\
                 Que no te vea: ponte tras las paredes.\n\
                 Los huesos van a la ceiba, a sus raíces.",
            en: "If you hear him close, he is far. If you hear him far, he is already here. \
                 Don't let him see you: get behind the walls. The bones go to the ceiba, to its roots.",
            by: both("— M."),
        },
        1 => Page {
            medium: Note,
            title: Words {
                es: "Clavada en el cobertizo",
                en: "Nailed up in the shed",
            },
            es: "Ají picante en el rincón. Lo detiene: se agacha a contar sus huesos y se olvida de ti.\n\
                 Tureco ladraba antes de que se apagara la luz. Lo dejé amarrado detrás de la casa: suéltenlo. \
                 A los perros sí les tiene miedo, y Tureco lo huele antes de que uno lo oiga.",
            en: "Hot peppers in the corner. They stop him: he crouches to count his bones and forgets you. \
                 Scatter one between you and him. Tureco was barking before the lights went out. I left him tied \
                 behind the house: set him loose. Dogs he does fear, and Tureco smells him before anyone hears him.",
            by: both("— Doña Rosa"),
        },
        2 => Page {
            medium: Note,
            title: Words {
                es: "Sobre el barril del corral",
                en: "On the corral barrel",
            },
            es: "El ganado se asusta con todo. Camina despacio junto a los corrales, agáchate, \
                 o se arma el escándalo y todo el llano lo oye.\n\
                 En el llano todo se escucha.",
            en: "The cattle spook at everything. Walk slowly past the pens, crouch, or there will be a racket \
                 and the whole llano will hear it. Out here everything is heard.",
            by: Words {
                es: "— el capataz",
                en: "— the foreman",
            },
        },
        3 => Page {
            medium: Note,
            title: Words {
                es: "En el altar de la ceiba",
                en: "On the ceiba's altar",
            },
            es: "Aquí también se escucha.\n\
                 Quien reza en voz alta calma el susto, pero la ceiba oye, y él también.\n\
                 Ponga los huesos en sus raíces, uno por uno.\n\
                 Y para la cajita de la llave: los números no están escritos. Los dice la radio de la casa, \
                 la Voz del Llano; la etiqueta del candado dice dónde buscarla en el dial.",
            en: "Here, too, he listens. Whoever prays aloud eases their fright, but the ceiba hears, and so does he. \
                 Lay the bones at its roots, one by one. \
                 And for the little key box: the numbers are not written down. The house radio says them, \
                 La Voz del Llano; the tag on the padlock says where to find it on the dial.",
            by: both("— la Madrina"),
        },
        4 => Page {
            medium: Logbook,
            title: Words {
                es: "En la cabina de la torre",
                en: "In the lookout cabin",
            },
            es: "Encendí el farol de la torre y vino derecho a la luz. Dura poco y tarda en poder encenderse otra vez.\n\
                 Desde arriba se ve todo el llano.\n\
                 El capataz no escribe los números del candado: los dicta la radio, pitido por pitido. \
                 Un pitido largo es un cero.",
            en: "I lit the tower lantern and he came straight to the light. It burns briefly and takes a long while \
                 before it can be lit again. From up here you can see the whole llano. A friend on the ground can work in peace. \
                 The foreman won't write the padlock's numbers down: the radio reads them out, pip by pip. \
                 A long tone is a zero.",
            by: Words {
                es: "— guardia, turno de noche",
                en: "— the night watch",
            },
        },
        5 => Page {
            medium: Note,
            title: Words {
                es: "En el palafito",
                en: "In the stilt hut",
            },
            es: "El agua esconde más que caminos. El vado del caño es lento y suena; los tablones crujen.\n\
                 Cada lluvia cambia el camino.",
            en: "The water hides more than paths. The ford is slow and loud; the planks creak. \
                 Every rain changes the way. Wading is quicker than the long bank road, but everything hears you.",
            by: Words {
                es: "— Elías, pescador",
                en: "— Elías, fisherman",
            },
        },
        6 => Page {
            medium: Note,
            title: Words {
                es: "En el cuarto del molino",
                en: "In the windmill shed",
            },
            es: "Sin la bomba no hay luz en la hacienda ni en el camino.\n\
                 Gira la manivela del molino: hace ruido, mucho ruido.\n\
                 Con luz, la camioneta arranca.",
            en: "Without the pump there is no light on the hacienda or the road. Crank the windmill's handle: \
                 it is loud, very loud. With power and the bones at rest, the truck will start, and it will roar.",
            by: Words {
                es: "— el mecánico",
                en: "— the mechanic",
            },
        },
        7 => Page {
            medium: Clipping,
            title: Words {
                es: "Diario de los Llanos, 16 de noviembre de 1963",
                en: "Diario de los Llanos, 16 November 1963",
            },
            es: "SAN JUAN DE LOS MORROS. — Tres obreros del Hato Santa Rosa, en el Guárico, no han regresado \
                 desde el aguacero del jueves. La Guardia Nacional recorrió el caño sin hallar rastro. \
                 Vecinos del sector aseguran haber oído silbidos «de noche, por la sabana», \
                 cosa que las autoridades atribuyen al viento y a la superstición.",
            en: "SAN JUAN DE LOS MORROS. — Three workers from the Santa Rosa ranch, in Guárico, have not returned \
                 since Thursday's downpour. The National Guard searched the creek and found no trace. \
                 Neighbours say they heard whistling \"at night, out on the savanna\", \
                 which the authorities put down to the wind and superstition.",
            by: Words {
                es: "— pegado en el portón",
                en: "— pasted up at the gate",
            },
        },
        8 => Page {
            medium: Letter,
            title: Words {
                es: "Carta de Doña Rosa a su hija",
                en: "Doña Rosa to her daughter",
            },
            es: "Mi niña: no vengas. La noche del aguacero el saco de él se enganchó en la cerca del caño y se rajó, \
                 y los huesos quedaron regados por todo el hato. Los muchachos los juntaron en cinco sacos de fique \
                 y los escondieron donde él no mirara: en la casa, en el corral, en el pajonal, en el caño y en la torre. \
                 La Madrina dice que hay que llevarlos a la ceiba, a las raíces, uno por uno. \
                 Cada saco que llegamos a poner lo puso más bravo. \
                 Si estás leyendo esto es porque volviste. Termina lo que nosotros no pudimos.",
            en: "My girl: don't come. The night of the downpour his sack caught on the caño fence and tore open, \
                 and the bones were scattered all over the ranch. The boys gathered them into five fibre sacks \
                 and hid them where he would not look: in the house, the corral, the tall grass, the caño and the tower. \
                 The Madrina says they must go to the ceiba, to its roots, one by one. \
                 Every sack we managed to lay there made him angrier. \
                 If you are reading this, you came back. Finish what we could not.",
            by: Words {
                es: "— tu mamá, Rosa",
                en: "— your mother, Rosa",
            },
        },
        9 => Page {
            medium: Photograph,
            title: Words {
                es: "Fiestas de la Cruz de Mayo, 1962",
                en: "May Cross festival, 1962",
            },
            es: "Santa Rosa, Cruz de Mayo del 62. De izquierda a derecha: el capataz, Rufino, Elías con el cuatro, \
                 la Madrina, Rosa y los muchachos. Tureco echado a los pies, como siempre, \
                 con la oreja parada hacia el llano.",
            en: "Santa Rosa, May Cross festival, '62. Left to right: the foreman, Rufino, Elías with his cuatro, \
                 the Madrina, Rosa and the boys. Tureco lying at our feet, as always, \
                 one ear pricked toward the llano.",
            by: Words {
                es: "— a lápiz, casi borrado",
                en: "— in pencil, nearly faded",
            },
        },
        10 => Page {
            medium: Radio,
            title: both("«Cuentos de camino», La Voz del Llano"),
            es: "…y la señora que nos escribe desde Calabozo dice que su abuelo lo conoció de muchacho: \
                 un malcriado que le exigió a su padre las asaduras de un venado. El padre volvió sin venado, \
                 y el muchacho lo mató y le sacó las asaduras a él, y se las dio a la madre para que las cocinara. \
                 Cuando la madre supo… (estática) …el abuelo lo amarró a un botalón, lo azotó con un chaparro \
                 y le echó ají en las heridas, y le soltó al perro. Y lo maldijo: cargarás los huesos de tu padre \
                 hasta el fin de los tiempos… (estática) …y si oyen el silbido, amigos, recuerden…",
            en: "…and the lady writing to us from Calabozo says her grandfather knew him as a boy: \
                 a spoiled boy who demanded his father bring him a deer's entrails. The father came home without a deer, \
                 and the boy killed him, took the entrails out of him instead, and gave them to his mother to cook. \
                 When the mother found out… (static) …the grandfather tied him to a post, whipped him with a chaparro switch, \
                 rubbed hot pepper into the wounds and set the dog on him. And he cursed him: you will carry your father's bones \
                 until the end of time… (static) …and if you hear the whistle, friends, remember…",
            by: Words {
                es: "— la radio del estante, entre la estática",
                en: "— the shelf radio, through the static",
            },
        },
        11 => Page {
            medium: Almanac,
            title: Words {
                es: "Almanaque de 1963, noviembre",
                en: "1963 almanac, November",
            },
            es: "Luna nueva el 15: no salir.\n\
                 Pilas para la linterna: se acaban cuando más falta hacen. Guardé pares en el porche, \
                 en el molino, en el corral, en el palafito, en el pajonal y en la torre.\n\
                 Apágala cuando lo oigas: la luz lo llama de lejos, más lejos de lo que te ve a ti.",
            en: "New moon on the 15th: don't go out.\n\
                 Batteries for the torch: they die just when you need them. I left pairs on the porch, \
                 at the windmill, the corral, the stilt hut, the tall grass and the tower.\n\
                 Switch it off when you hear him: the light calls him from far off, farther than he can see you.",
            by: Words {
                es: "— letra de Rufino",
                en: "— Rufino's hand",
            },
        },
        12 => Page {
            medium: Ledger,
            title: Words {
                es: "Libreta del molino",
                en: "Windmill notebook",
            },
            es: "La bomba tiene su compás. Si fuerzas la manivela a destiempo, chilla como animal herido \
                 y ese chillido se oye hasta la ceiba. Oye el toque, espera la marca y dale en su momento. \
                 Lo mismo el arranque de la camioneta. Y la Madrina dice que los huesos también se ponen con compás.",
            en: "The pump has its rhythm. Force the handle off the beat and it screams like a wounded animal, \
                 and that scream carries all the way to the ceiba. Hear the tap, wait for the mark and push on time. \
                 The truck's starter is the same. And the Madrina says the bones must be laid down with rhythm too.",
            by: Words {
                es: "— el mecánico",
                en: "— the mechanic",
            },
        },
        13 => Page {
            medium: Copla,
            title: Words {
                es: "Copla del caminante",
                en: "The walker's copla",
            },
            es: "Por la sabana de noche / va silbando un caminante: / si lo escuchas bien cerquita, / ya se fue pa' lo distante.\n\
                 Si lo oyes lejos, compadre, / no lo pienses, ya está aquí; / lleva en el saco los huesos / del padre que hizo morir.\n\
                 Su abuelo lo amarró al palo, / le dio con el chaparrón, / le echó ají en las heridas / y lo mordió el perro Tureco… \
                 ¡que no rima, pero es verdad!",
            en: "Across the savanna by night / a walker goes whistling his way: / if you hear him close beside you, / he's already far away.\n\
                 If you hear him far, my friend, / don't stop to think — he's here; / in his sack he carries the bones / of the father he killed that year.\n\
                 His grandfather tied him to the post, / whipped him with the chaparro switch, / rubbed pepper in the wounds, / and Tureco the dog bit him… \
                 (it doesn't rhyme, but it's true!)",
            by: Words {
                es: "— escrita en la etiqueta de un saco",
                en: "— written on a sack tag",
            },
        },
        14 => Page {
            medium: Telegram,
            title: both("Guardia Nacional, San Juan de los Morros"),
            es: "HATO SANTA ROSA STOP CAMINO REAL INUNDADO DESDE EL KILOMETRO 12 STOP \
                 BUSQUEDA SUSPENDIDA HASTA QUE BAJEN LAS AGUAS STOP NO SE ENVIARAN MAS HOMBRES STOP \
                 LA CAMIONETA DEL HATO ES LA UNICA SALIDA STOP",
            en: "SANTA ROSA RANCH STOP MAIN ROAD FLOODED FROM KILOMETRE 12 STOP \
                 SEARCH SUSPENDED UNTIL THE WATERS GO DOWN STOP NO MORE MEN WILL BE SENT STOP \
                 THE RANCH TRUCK IS THE ONLY WAY OUT STOP",
            by: Words {
                es: "— entregado el 17 de noviembre",
                en: "— delivered 17 November",
            },
        },
        15 => Page {
            medium: Register,
            title: Words {
                es: "Libro de difuntos, 1871 (copia)",
                en: "Book of the dead, 1871 (copy)",
            },
            es: "«Se dio cristiana sepultura a lo que se halló de un hombre a quien su propio hijo quitó la vida \
                 por las asaduras de un venado. El abuelo del mozo lo castigó por su mano y lo maldijo \
                 a cargar los huesos del padre hasta el fin de los tiempos. El mozo huyó por la sabana silbando.»\n\
                 Al pie, de otra letra: los huesos no descansan hasta volver a la raíz donde nació el padre: \
                 la ceiba de Santa Rosa. Cuando estén todos, que alguien se vaya antes de que amanezca.",
            en: "\"Christian burial was given to what was found of a man whose own son took his life \
                 for a deer's entrails. The young man's grandfather punished him with his own hand and cursed him \
                 to carry his father's bones until the end of time. The young man fled across the savanna, whistling.\"\n\
                 Below, in another hand: the bones will not rest until they return to the root where the father was born: \
                 the Santa Rosa ceiba. When they are all there, someone must leave before dawn.",
            by: Words {
                es: "— copiado por la Madrina",
                en: "— copied out by the Madrina",
            },
        },
        16 => Page {
            medium: Ledger,
            title: Words {
                es: "Cuaderno del capataz",
                en: "The foreman's ledger",
            },
            es: "12 nov. — Otra res amaneció con los ojos blancos.\n\
                 13 nov. — Juntamos los huesos en cinco sacos. Pesan más de lo que deberían.\n\
                 14 nov. — Llevamos dos a la ceiba. Desde entonces silba más seguido y ya no espera.\n\
                 15 nov. — Rufino no volvió del caño.\n\
                 16 nov. — Dejé la camioneta en el puente, lista. La llave, en la cajita sobre las cajas del molino, con candado de tres \
                 números. No los apunto, que él lee: los repite la radio del estante, en la frecuencia de la etiqueta del \
                 candado. Si truena, hay que esperar la vuelta siguiente. Cuando ronque el motor, él viene: hay que aguantar a que caliente y subirse todos. Todos.",
            en: "12 Nov. — Another steer woke up with white eyes.\n\
                 13 Nov. — We gathered the bones into five sacks. They weigh more than they should.\n\
                 14 Nov. — We took two to the ceiba. Since then he whistles more often and no longer waits.\n\
                 15 Nov. — Rufino did not come back from the caño.\n\
                 16 Nov. — I left the truck at the bridge, ready. The key is in the little box on the crates by the windmill, under a \
                 three-number padlock. I won't write them down, he reads: the shelf radio repeats them, on the frequency on \
                 the padlock's tag. If it thunders, wait for the next round. When the engine roars he comes: you have to hold out until it warms up, and everyone get on. Everyone.",
            by: Words {
                es: "— el capataz",
                en: "— the foreman",
            },
        },
        17 => Page {
            medium: Logbook,
            title: Words {
                es: "Libro de guardia, torre vigía",
                en: "Watch log, lookout tower",
            },
            es: "Madrugada del 15. Desde arriba vi una luz que se movía en el pajonal, despacio, como quien busca. \
                 Conté a los nuestros: estábamos todos aquí. \
                 Si ven una linterna lejos y no saben de quién es, no le hagan señas.",
            en: "Small hours of the 15th. From up here I saw a light moving through the tall grass, slowly, like someone searching. \
                 I counted ours: we were all up here. \
                 If you see a torch far off and don't know whose it is, don't signal to it.",
            by: Words {
                es: "— guardia, turno de noche",
                en: "— the night watch",
            },
        },
        18 => Page {
            medium: PrayerCard,
            title: Words {
                es: "Oración del caminante",
                en: "The walker's prayer",
            },
            es: "Ánimas benditas del purgatorio, que el que silba no me encuentre. \
                 Que mis pasos sean de gato y mi luz no lo despierte. \
                 Que si me ve, encuentre una pared entre los dos; \
                 que si me alcanza, encuentre ají en el camino; \
                 y que cuente sus huesos hasta que yo esté lejos. Amén.",
            en: "Blessed souls of purgatory, let the one who whistles not find me. \
                 Let my steps be a cat's and my light not wake him. \
                 If he sees me, let him find a wall between us; \
                 if he reaches me, let him find pepper on the road; \
                 and let him count his bones until I am far away. Amen.",
            by: Words {
                es: "— estampita dejada en la camioneta",
                en: "— a card left in the truck",
            },
        },
        19 => Page {
            medium: Register,
            title: Words {
                es: "Las tres vueltas",
                en: "The three returns",
            },
            es: "Vuelve de tres maneras, decían los viejos.\n\
                 El del Borracho: oye hasta el paso de un gato y silba enredado, pero anda torcido; donde anduvo \
                 suenan botellas.\n\
                 El Hijo mismo: llora cuando le ponen los huesos al padre y cada hueso lo enfurece más, pero se \
                 queda más rato contándolos.\n\
                 El Arriero: el ganado brama cuando él pasa, restalla un látigo en lo oscuro y anda más ligero.\n\
                 Con todos los huesos en la ceiba, nómbralo bien y descansará. Nómbralo mal y vendrá por ti.",
            en: "He comes back in three ways, the old ones said.\n\
                 The Drunkard's: he hears even a cat's step and whistles slurred, but walks crooked; where he has \
                 been, bottles clink.\n\
                 The Son himself: he weeps when his father's bones are laid down, and each bone angers him more, \
                 but he lingers longer counting them.\n\
                 The Drover: the cattle bellow as he passes, a whip cracks in the dark, and he walks faster.\n\
                 With every bone at the ceiba, name him rightly and he will rest. Name him wrongly and he will come for you.",
            by: Words {
                es: "— hoja suelta, letra de la Madrina",
                en: "— a loose page, the Madrina's hand",
            },
        },
        _ => Page {
            medium: Note,
            title: both(""),
            es: "",
            en: "",
            by: both(""),
        },
    }
}

/// La Madrina's tale, one chapter for each bundle laid at the ceiba and a
/// last one when all the bones are home. The last line of each hints at
/// what he learns as his anger grows.
pub struct Chapter {
    pub title: Words,
    pub es: &'static str,
    pub en: &'static str,
}

impl Chapter {
    /// The chapter's words, in one language.
    pub fn text(&self, lang: Lang) -> &'static str {
        lang.pick(self.en, self.es)
    }
}

/// How many chapters the Madrina tells.
pub const CHAPTERS: u8 = 6;

/// Chapter `n` (1..=`CHAPTERS`).
pub fn chapter(n: u8) -> Option<Chapter> {
    let c = |title_es: &'static str, title_en: &'static str, es: &'static str, en: &'static str| {
        Some(Chapter {
            title: Words {
                es: title_es,
                en: title_en,
            },
            es,
            en,
        })
    };
    match n {
        1 => c(
            "I · Las asaduras",
            "I · The entrails",
            "Era un muchacho malcriado. Le pidió a su padre las asaduras de un venado, y no quiso otra cosa.\n\
             Cada hueso que vuelve a la ceiba lo despierta más: silbará más seguido.",
            "He was a spoiled boy. He demanded a deer's entrails from his father, and would have nothing else.\n\
             Every bone that comes home to the ceiba wakes him more: he will whistle more often.",
        ),
        2 => c(
            "II · El padre",
            "II · The father",
            "El padre volvió sin venado. El muchacho lo mató, le sacó las asaduras y se las llevó a su madre para que las cocinara.\n\
             Ahora una linterna encendida lo llama desde más lejos.",
            "The father came home without a deer. The boy killed him, took out his entrails and brought them to his mother to cook.\n\
             Now a lit torch calls him from farther away.",
        ),
        3 => c(
            "III · El botalón",
            "III · The post",
            "El abuelo lo amarró a un botalón, lo azotó con un chaparro, le echó ají en las heridas y le soltó al perro.\n\
             Ya conoce el pajonal: entre la paja alta te encuentra desde más lejos.",
            "The grandfather tied him to a post, whipped him with a chaparro switch, rubbed hot pepper into the wounds \
             and set the dog on him.\n\
             He knows the tall grass now: he finds you in it from farther off.",
        ),
        4 => c(
            "IV · La maldición",
            "IV · The curse",
            "Y lo maldijo: cargarás los huesos de tu padre en un saco, para siempre, y silbarás para que todos sepan que vienes.\n\
             Se le acaba la paciencia: cada hueso lo enfurece más.",
            "And he cursed him: you will carry your father's bones in a sack, forever, and whistle so all will know you are coming.\n\
             His patience is running out: every bone angers him more.",
        ),
        5 => c(
            "V · Noche de tormenta",
            "V · The storm night",
            "En Santa Rosa lo oímos una noche de tormenta, y los huesos se regaron por todo el hato.\n\
             Cuando ronque el motor, vendrá: el motor lo llama.",
            "At Santa Rosa we heard him on a storm night, and the bones were scattered all over the ranch.\n\
             When the engine roars he will come: the engine calls him.",
        ),
        6 => c(
            "VI · Todos en casa",
            "VI · All home",
            "Todos los huesos están en la ceiba. Ahora, antes del alba, nómbralo bien.",
            "All the bones are at the ceiba. Now, before dawn, name him rightly.",
        ),
        _ => None,
    }
}

/// How many chapters have been told once `delivered` of `total` bundles lie
/// at the ceiba: one for each bundle laid, and the last when all are home.
pub fn chapters_told(delivered: u8, total: u8) -> u8 {
    if total > 0 && delivered >= total {
        CHAPTERS
    } else {
        delivered.min(CHAPTERS - 1)
    }
}

/// The lettering on a painted board, one string per line. Capitals and
/// plain ASCII only: the boards are stencilled with a 5x7 face.
pub fn sign(id: u8) -> &'static [&'static str] {
    match id {
        0 => &[
            "PROPIEDAD PRIVADA",
            "SOLO PERSONAL AUTORIZADO",
            "EL SILBON TAMBIEN VIGILA +",
        ],
        1 => &["GANADERIA", "NUESTRA VIDA"],
        2 => &["HACIENDA", "SANTA ROSA +"],
        3 => &["AGUA", "VIDA", "LLANOS"],
        4 => &["AQUI TAMBIEN", "SE ESCUCHA +"],
        5 => &["GANADO", "TRABAJO", "NUESTRA TIERRA"],
        6 => &["AGUAS PROFUNDAS", "<- NO PASAR"],
        7 => &["EL CAMINO", "CAMBIA", "CADA LLUVIA"],
        8 => &["LA SALIDA", "RIO 6 KM"],
        9 => &["SAN JUAN DE", "LOS MORROS", "30 KM"],
        10 => &["EL AGUA ESCONDE", "MAS QUE CAMINOS"],
        _ => &["TORRE VIGIA", "NO SUBIR SOLO"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_never_tells_a_nights_code() {
        // The radio reads the code out; no page or chapter has a blank for a
        // night's digit, so what the journal keeps is the same every night.
        for id in 0..PAGES {
            let page = note(id);
            let texts = [page.title.es, page.title.en, page.es, page.en, page.by.es, page.by.en];
            for text in texts {
                assert!(!text.contains('{'), "page {id} still has a blank for a digit");
            }
        }
        for n in 1..=CHAPTERS {
            let c = chapter(n).expect("every chapter is written");
            for text in [c.title.es, c.title.en, c.es, c.en] {
                assert!(!text.is_empty() && !text.contains('{'), "chapter {n}");
            }
        }
        assert!(chapter(0).is_none() && chapter(CHAPTERS + 1).is_none());
    }

    #[test]
    fn each_language_reads_a_whole_page_of_its_own() {
        // A reader in either language gets the whole page in it, and never
        // the other language's page beside it.
        for id in 0..PAGES {
            let page = note(id);
            let (en, es) = (page.text(Lang::En), page.text(Lang::Es));
            assert!(!en.is_empty() && !es.is_empty() && en != es, "page {id}");
            assert!(!en.contains(es) && !es.contains(en), "page {id}");
            for lang in [Lang::En, Lang::Es] {
                assert!(!lang.say(page.title).is_empty(), "page {id}");
                assert!(!page.medium.label(lang).is_empty(), "page {id}");
            }
        }
        for n in 1..=CHAPTERS {
            let c = chapter(n).expect("every chapter is written");
            assert_ne!(c.text(Lang::En), c.text(Lang::Es), "chapter {n}");
            assert_ne!(Lang::En.say(c.title), Lang::Es.say(c.title), "chapter {n}");
        }
    }

    #[test]
    fn each_bundle_laid_tells_the_next_chapter_and_all_home_the_last() {
        let total = 5;
        assert_eq!(chapters_told(0, total), 0);
        for laid in 1..total {
            assert_eq!(chapters_told(laid, total), chapters_told(laid - 1, total) + 1);
        }
        assert_eq!(chapters_told(total, total), CHAPTERS, "all home: the tale ends");
        assert_eq!(chapters_told(0, 0), 0);
    }
}
