//! The notes left around the hacienda: fiction inspired by the legend, each
//! one a diegetic hint for a mechanic. Spanish first, English beneath.

/// One handwritten page.
pub struct Note {
    pub es: &'static str,
    pub en: &'static str,
    pub by: &'static str,
}

/// The page for a note id from the layout (0 is the one on the table).
pub fn note(id: u8) -> Note {
    match id {
        0 => Note {
            es: "Si lo oyes cerca, está lejos.\n\
                 Si lo oyes lejos, ya está aquí.\n\
                 Que no te vea: ponte tras las paredes.\n\
                 Los huesos van a la ceiba, a sus raíces.",
            en: "If you hear him close, he is far. If you hear him far, he is already here. \
                 Don't let him see you: get behind the walls. The bones go to the ceiba, to its roots.",
            by: "— M.",
        },
        1 => Note {
            es: "Ají picante en el rincón. Es lo único que lo detiene: se agacha a contar sus huesos y se olvida de ti.\n\
                 Tureco ladraba antes de que se apagara la luz.",
            en: "Hot peppers in the corner. It is the only thing that stops him: he crouches to count his bones \
                 and forgets you. Scatter one between you and him. Tureco was barking before the lights went out.",
            by: "— Doña Rosa",
        },
        2 => Note {
            es: "El ganado se asusta con todo. Camina despacio junto a los corrales, agáchate, \
                 o se arma el escándalo y todo el llano lo oye.\n\
                 En el llano todo se escucha.",
            en: "The cattle spook at everything. Walk slowly past the pens, crouch, or there will be a racket \
                 and the whole llano will hear it. Out here everything is heard.",
            by: "— el capataz",
        },
        3 => Note {
            es: "Aquí también se escucha.\n\
                 Quien reza en voz alta calma el susto, pero la ceiba oye, y él también.\n\
                 Ponga los huesos en sus raíces, uno por uno.",
            en: "Here, too, he listens. Whoever prays aloud eases their fright, but the ceiba hears, and so does he. \
                 Lay the bones at its roots, one by one.",
            by: "— la Madrina",
        },
        4 => Note {
            es: "Encendí el farol de la torre y vino derecho a la luz. Dura poco y tarda en poder encenderse otra vez.\n\
                 Desde arriba se ve todo el llano.",
            en: "I lit the tower lantern and he came straight to the light. It burns briefly and takes a long while \
                 before it can be lit again. From up here you can see the whole llano. A friend on the ground can work in peace.",
            by: "— guardia, turno de noche",
        },
        5 => Note {
            es: "El agua esconde más que caminos. El vado del caño es lento y suena; los tablones crujen.\n\
                 Cada lluvia cambia el camino.",
            en: "The water hides more than paths. The ford is slow and loud; the planks creak. \
                 Every rain changes the way. Wading is quicker than the long bank road, but everything hears you.",
            by: "— Elías, pescador",
        },
        _ => Note {
            es: "Sin la bomba no hay luz en la hacienda ni en el camino.\n\
                 Gira la manivela del molino: hace ruido, mucho ruido.\n\
                 Con luz, la camioneta arranca.",
            en: "Without the pump there is no light on the hacienda or the road. Crank the windmill's handle: \
                 it is loud, very loud. With power and the bones at rest, the truck will start, and it will roar.",
            by: "— el mecánico",
        },
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
