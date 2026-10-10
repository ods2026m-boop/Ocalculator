//! Periodic table data (standard atomic weights, g/mol).
//!
//! Weights follow the IUPAC 2021 conventional values; synthetic elements
//! without a stable isotope use the mass number of the most stable known
//! isotope in brackets (stored as a plain `f64`).

/// One element: symbol, name, atomic number, standard atomic weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementData {
    /// Symbol, e.g. `"H"`.
    pub symbol: &'static str,
    /// Name, e.g. `"Hydrogen"`.
    pub name: &'static str,
    /// Atomic number.
    pub number: u32,
    /// Standard atomic weight (g/mol).
    pub mass: f64,
}

/// All 118 elements in atomic-number order.
pub const TABLE: [ElementData; 118] = [
    ElementData {
        symbol: "H",
        name: "Hydrogen",
        number: 1,
        mass: 1.008,
    },
    ElementData {
        symbol: "He",
        name: "Helium",
        number: 2,
        mass: 4.0026,
    },
    ElementData {
        symbol: "Li",
        name: "Lithium",
        number: 3,
        mass: 6.94,
    },
    ElementData {
        symbol: "Be",
        name: "Beryllium",
        number: 4,
        mass: 9.0122,
    },
    ElementData {
        symbol: "B",
        name: "Boron",
        number: 5,
        mass: 10.81,
    },
    ElementData {
        symbol: "C",
        name: "Carbon",
        number: 6,
        mass: 12.011,
    },
    ElementData {
        symbol: "N",
        name: "Nitrogen",
        number: 7,
        mass: 14.007,
    },
    ElementData {
        symbol: "O",
        name: "Oxygen",
        number: 8,
        mass: 15.999,
    },
    ElementData {
        symbol: "F",
        name: "Fluorine",
        number: 9,
        mass: 18.998,
    },
    ElementData {
        symbol: "Ne",
        name: "Neon",
        number: 10,
        mass: 20.180,
    },
    ElementData {
        symbol: "Na",
        name: "Sodium",
        number: 11,
        mass: 22.990,
    },
    ElementData {
        symbol: "Mg",
        name: "Magnesium",
        number: 12,
        mass: 24.305,
    },
    ElementData {
        symbol: "Al",
        name: "Aluminium",
        number: 13,
        mass: 26.982,
    },
    ElementData {
        symbol: "Si",
        name: "Silicon",
        number: 14,
        mass: 28.085,
    },
    ElementData {
        symbol: "P",
        name: "Phosphorus",
        number: 15,
        mass: 30.974,
    },
    ElementData {
        symbol: "S",
        name: "Sulfur",
        number: 16,
        mass: 32.06,
    },
    ElementData {
        symbol: "Cl",
        name: "Chlorine",
        number: 17,
        mass: 35.45,
    },
    ElementData {
        symbol: "Ar",
        name: "Argon",
        number: 18,
        mass: 39.948,
    },
    ElementData {
        symbol: "K",
        name: "Potassium",
        number: 19,
        mass: 39.098,
    },
    ElementData {
        symbol: "Ca",
        name: "Calcium",
        number: 20,
        mass: 40.078,
    },
    ElementData {
        symbol: "Sc",
        name: "Scandium",
        number: 21,
        mass: 44.956,
    },
    ElementData {
        symbol: "Ti",
        name: "Titanium",
        number: 22,
        mass: 47.867,
    },
    ElementData {
        symbol: "V",
        name: "Vanadium",
        number: 23,
        mass: 50.942,
    },
    ElementData {
        symbol: "Cr",
        name: "Chromium",
        number: 24,
        mass: 51.996,
    },
    ElementData {
        symbol: "Mn",
        name: "Manganese",
        number: 25,
        mass: 54.938,
    },
    ElementData {
        symbol: "Fe",
        name: "Iron",
        number: 26,
        mass: 55.845,
    },
    ElementData {
        symbol: "Co",
        name: "Cobalt",
        number: 27,
        mass: 58.933,
    },
    ElementData {
        symbol: "Ni",
        name: "Nickel",
        number: 28,
        mass: 58.693,
    },
    ElementData {
        symbol: "Cu",
        name: "Copper",
        number: 29,
        mass: 63.546,
    },
    ElementData {
        symbol: "Zn",
        name: "Zinc",
        number: 30,
        mass: 65.38,
    },
    ElementData {
        symbol: "Ga",
        name: "Gallium",
        number: 31,
        mass: 69.723,
    },
    ElementData {
        symbol: "Ge",
        name: "Germanium",
        number: 32,
        mass: 72.630,
    },
    ElementData {
        symbol: "As",
        name: "Arsenic",
        number: 33,
        mass: 74.922,
    },
    ElementData {
        symbol: "Se",
        name: "Selenium",
        number: 34,
        mass: 78.971,
    },
    ElementData {
        symbol: "Br",
        name: "Bromine",
        number: 35,
        mass: 79.904,
    },
    ElementData {
        symbol: "Kr",
        name: "Krypton",
        number: 36,
        mass: 83.798,
    },
    ElementData {
        symbol: "Rb",
        name: "Rubidium",
        number: 37,
        mass: 85.468,
    },
    ElementData {
        symbol: "Sr",
        name: "Strontium",
        number: 38,
        mass: 87.62,
    },
    ElementData {
        symbol: "Y",
        name: "Yttrium",
        number: 39,
        mass: 88.906,
    },
    ElementData {
        symbol: "Zr",
        name: "Zirconium",
        number: 40,
        mass: 91.224,
    },
    ElementData {
        symbol: "Nb",
        name: "Niobium",
        number: 41,
        mass: 92.906,
    },
    ElementData {
        symbol: "Mo",
        name: "Molybdenum",
        number: 42,
        mass: 95.95,
    },
    ElementData {
        symbol: "Tc",
        name: "Technetium",
        number: 43,
        mass: 98.0,
    },
    ElementData {
        symbol: "Ru",
        name: "Ruthenium",
        number: 44,
        mass: 101.07,
    },
    ElementData {
        symbol: "Rh",
        name: "Rhodium",
        number: 45,
        mass: 102.91,
    },
    ElementData {
        symbol: "Pd",
        name: "Palladium",
        number: 46,
        mass: 106.42,
    },
    ElementData {
        symbol: "Ag",
        name: "Silver",
        number: 47,
        mass: 107.87,
    },
    ElementData {
        symbol: "Cd",
        name: "Cadmium",
        number: 48,
        mass: 112.41,
    },
    ElementData {
        symbol: "In",
        name: "Indium",
        number: 49,
        mass: 114.82,
    },
    ElementData {
        symbol: "Sn",
        name: "Tin",
        number: 50,
        mass: 118.71,
    },
    ElementData {
        symbol: "Sb",
        name: "Antimony",
        number: 51,
        mass: 121.76,
    },
    ElementData {
        symbol: "Te",
        name: "Tellurium",
        number: 52,
        mass: 127.60,
    },
    ElementData {
        symbol: "I",
        name: "Iodine",
        number: 53,
        mass: 126.90,
    },
    ElementData {
        symbol: "Xe",
        name: "Xenon",
        number: 54,
        mass: 131.29,
    },
    ElementData {
        symbol: "Cs",
        name: "Caesium",
        number: 55,
        mass: 132.91,
    },
    ElementData {
        symbol: "Ba",
        name: "Barium",
        number: 56,
        mass: 137.33,
    },
    ElementData {
        symbol: "La",
        name: "Lanthanum",
        number: 57,
        mass: 138.91,
    },
    ElementData {
        symbol: "Ce",
        name: "Cerium",
        number: 58,
        mass: 140.12,
    },
    ElementData {
        symbol: "Pr",
        name: "Praseodymium",
        number: 59,
        mass: 140.91,
    },
    ElementData {
        symbol: "Nd",
        name: "Neodymium",
        number: 60,
        mass: 144.24,
    },
    ElementData {
        symbol: "Pm",
        name: "Promethium",
        number: 61,
        mass: 145.0,
    },
    ElementData {
        symbol: "Sm",
        name: "Samarium",
        number: 62,
        mass: 150.36,
    },
    ElementData {
        symbol: "Eu",
        name: "Europium",
        number: 63,
        mass: 151.96,
    },
    ElementData {
        symbol: "Gd",
        name: "Gadolinium",
        number: 64,
        mass: 157.25,
    },
    ElementData {
        symbol: "Tb",
        name: "Terbium",
        number: 65,
        mass: 158.93,
    },
    ElementData {
        symbol: "Dy",
        name: "Dysprosium",
        number: 66,
        mass: 162.50,
    },
    ElementData {
        symbol: "Ho",
        name: "Holmium",
        number: 67,
        mass: 164.93,
    },
    ElementData {
        symbol: "Er",
        name: "Erbium",
        number: 68,
        mass: 167.26,
    },
    ElementData {
        symbol: "Tm",
        name: "Thulium",
        number: 69,
        mass: 168.93,
    },
    ElementData {
        symbol: "Yb",
        name: "Ytterbium",
        number: 70,
        mass: 173.05,
    },
    ElementData {
        symbol: "Lu",
        name: "Lutetium",
        number: 71,
        mass: 174.97,
    },
    ElementData {
        symbol: "Hf",
        name: "Hafnium",
        number: 72,
        mass: 178.49,
    },
    ElementData {
        symbol: "Ta",
        name: "Tantalum",
        number: 73,
        mass: 180.95,
    },
    ElementData {
        symbol: "W",
        name: "Tungsten",
        number: 74,
        mass: 183.84,
    },
    ElementData {
        symbol: "Re",
        name: "Rhenium",
        number: 75,
        mass: 186.21,
    },
    ElementData {
        symbol: "Os",
        name: "Osmium",
        number: 76,
        mass: 190.23,
    },
    ElementData {
        symbol: "Ir",
        name: "Iridium",
        number: 77,
        mass: 192.22,
    },
    ElementData {
        symbol: "Pt",
        name: "Platinum",
        number: 78,
        mass: 195.08,
    },
    ElementData {
        symbol: "Au",
        name: "Gold",
        number: 79,
        mass: 196.97,
    },
    ElementData {
        symbol: "Hg",
        name: "Mercury",
        number: 80,
        mass: 200.59,
    },
    ElementData {
        symbol: "Tl",
        name: "Thallium",
        number: 81,
        mass: 204.38,
    },
    ElementData {
        symbol: "Pb",
        name: "Lead",
        number: 82,
        mass: 207.2,
    },
    ElementData {
        symbol: "Bi",
        name: "Bismuth",
        number: 83,
        mass: 208.98,
    },
    ElementData {
        symbol: "Po",
        name: "Polonium",
        number: 84,
        mass: 209.0,
    },
    ElementData {
        symbol: "At",
        name: "Astatine",
        number: 85,
        mass: 210.0,
    },
    ElementData {
        symbol: "Rn",
        name: "Radon",
        number: 86,
        mass: 222.0,
    },
    ElementData {
        symbol: "Fr",
        name: "Francium",
        number: 87,
        mass: 223.0,
    },
    ElementData {
        symbol: "Ra",
        name: "Radium",
        number: 88,
        mass: 226.0,
    },
    ElementData {
        symbol: "Ac",
        name: "Actinium",
        number: 89,
        mass: 227.0,
    },
    ElementData {
        symbol: "Th",
        name: "Thorium",
        number: 90,
        mass: 232.04,
    },
    ElementData {
        symbol: "Pa",
        name: "Protactinium",
        number: 91,
        mass: 231.04,
    },
    ElementData {
        symbol: "U",
        name: "Uranium",
        number: 92,
        mass: 238.03,
    },
    ElementData {
        symbol: "Np",
        name: "Neptunium",
        number: 93,
        mass: 237.0,
    },
    ElementData {
        symbol: "Pu",
        name: "Plutonium",
        number: 94,
        mass: 244.0,
    },
    ElementData {
        symbol: "Am",
        name: "Americium",
        number: 95,
        mass: 243.0,
    },
    ElementData {
        symbol: "Cm",
        name: "Curium",
        number: 96,
        mass: 247.0,
    },
    ElementData {
        symbol: "Bk",
        name: "Berkelium",
        number: 97,
        mass: 247.0,
    },
    ElementData {
        symbol: "Cf",
        name: "Californium",
        number: 98,
        mass: 251.0,
    },
    ElementData {
        symbol: "Es",
        name: "Einsteinium",
        number: 99,
        mass: 252.0,
    },
    ElementData {
        symbol: "Fm",
        name: "Fermium",
        number: 100,
        mass: 257.0,
    },
    ElementData {
        symbol: "Md",
        name: "Mendelevium",
        number: 101,
        mass: 258.0,
    },
    ElementData {
        symbol: "No",
        name: "Nobelium",
        number: 102,
        mass: 259.0,
    },
    ElementData {
        symbol: "Lr",
        name: "Lawrencium",
        number: 103,
        mass: 266.0,
    },
    ElementData {
        symbol: "Rf",
        name: "Rutherfordium",
        number: 104,
        mass: 267.0,
    },
    ElementData {
        symbol: "Db",
        name: "Dubnium",
        number: 105,
        mass: 268.0,
    },
    ElementData {
        symbol: "Sg",
        name: "Seaborgium",
        number: 106,
        mass: 269.0,
    },
    ElementData {
        symbol: "Bh",
        name: "Bohrium",
        number: 107,
        mass: 270.0,
    },
    ElementData {
        symbol: "Hs",
        name: "Hassium",
        number: 108,
        mass: 277.0,
    },
    ElementData {
        symbol: "Mt",
        name: "Meitnerium",
        number: 109,
        mass: 278.0,
    },
    ElementData {
        symbol: "Ds",
        name: "Darmstadtium",
        number: 110,
        mass: 281.0,
    },
    ElementData {
        symbol: "Rg",
        name: "Roentgenium",
        number: 111,
        mass: 282.0,
    },
    ElementData {
        symbol: "Cn",
        name: "Copernicium",
        number: 112,
        mass: 285.0,
    },
    ElementData {
        symbol: "Nh",
        name: "Nihonium",
        number: 113,
        mass: 286.0,
    },
    ElementData {
        symbol: "Fl",
        name: "Flerovium",
        number: 114,
        mass: 289.0,
    },
    ElementData {
        symbol: "Mc",
        name: "Moscovium",
        number: 115,
        mass: 290.0,
    },
    ElementData {
        symbol: "Lv",
        name: "Livermorium",
        number: 116,
        mass: 293.0,
    },
    ElementData {
        symbol: "Ts",
        name: "Tennessine",
        number: 117,
        mass: 294.0,
    },
    ElementData {
        symbol: "Og",
        name: "Oganesson",
        number: 118,
        mass: 294.0,
    },
];

/// Look up an element by symbol (case-sensitive: `He` ≠ `HE`) or name
/// (case-insensitive, e.g. `"oxygen"`).
#[must_use]
pub fn lookup(symbol_or_name: &str) -> Option<ElementData> {
    TABLE
        .iter()
        .find(|e| e.symbol == symbol_or_name)
        .copied()
        .or_else(|| {
            let lower = symbol_or_name.to_ascii_lowercase();
            TABLE
                .iter()
                .find(|e| e.name.to_ascii_lowercase() == lower)
                .copied()
        })
}

/// Look up an element by atomic number (1–118).
#[must_use]
pub fn by_number(number: u32) -> Option<ElementData> {
    TABLE.iter().find(|e| e.number == number).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups() {
        assert_eq!(lookup("H").unwrap().mass, 1.008);
        assert_eq!(lookup("oxygen").unwrap().symbol, "O");
        assert_eq!(by_number(6).unwrap().symbol, "C");
        assert!(lookup("Xx").is_none());
        assert!(lookup("").is_none());
        assert!(lookup("HE").is_none());
    }

    #[test]
    fn table_covers_all_elements() {
        assert_eq!(TABLE.len(), 118);
        for (i, e) in TABLE.iter().enumerate() {
            assert_eq!(e.number, i as u32 + 1, "gap at {e:?}");
            assert!(e.mass > 0.0);
        }
    }
}
