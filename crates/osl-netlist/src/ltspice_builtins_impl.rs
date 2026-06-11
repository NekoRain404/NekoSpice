fn ltspice_builtin_symbol(name: &str) -> Option<LtspiceSymbolSpec> {
    const RES_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 16 }, AscPoint { x: 16, y: 96 }];
    const CAP_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 0 }, AscPoint { x: 16, y: 64 }];
    const SOURCE_PINS: &[AscPoint] = &[AscPoint { x: 0, y: 16 }, AscPoint { x: 0, y: 96 }];
    const CURRENT_PINS: &[AscPoint] = &[AscPoint { x: 0, y: 0 }, AscPoint { x: 0, y: 80 }];
    const DIODE_PINS: &[AscPoint] = &[AscPoint { x: 16, y: 0 }, AscPoint { x: 16, y: 64 }];
    const BJT_PINS: &[AscPoint] = &[
        AscPoint { x: 64, y: 0 },
        AscPoint { x: 0, y: 48 },
        AscPoint { x: 64, y: 96 },
    ];
    const MOS_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 80 },
        AscPoint { x: 48, y: 96 },
    ];
    const MOS4_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 80 },
        AscPoint { x: 48, y: 96 },
        AscPoint { x: 48, y: 48 },
    ];
    const JFET_PINS: &[AscPoint] = &[
        AscPoint { x: 48, y: 0 },
        AscPoint { x: 0, y: 64 },
        AscPoint { x: 48, y: 96 },
    ];
    const E_SOURCE_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 16 },
        AscPoint { x: 0, y: 96 },
        AscPoint { x: -48, y: 32 },
        AscPoint { x: -48, y: 80 },
    ];
    const G_SOURCE_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 96 },
        AscPoint { x: 0, y: 16 },
        AscPoint { x: -48, y: 32 },
        AscPoint { x: -48, y: 80 },
    ];
    const SWITCH_PINS: &[AscPoint] = &[
        AscPoint { x: 0, y: 16 },
        AscPoint { x: 0, y: 96 },
        AscPoint { x: -48, y: 80 },
        AscPoint { x: -48, y: 32 },
    ];

    match ltspice_symbol_basename(name).as_str() {
        "res" | "res2" => Some(LtspiceSymbolSpec {
            prefix: "R".to_string(),
            pins: RES_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "cap" | "polcap" => Some(LtspiceSymbolSpec {
            prefix: "C".to_string(),
            pins: CAP_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "ind" | "ind2" => Some(LtspiceSymbolSpec {
            prefix: "L".to_string(),
            pins: RES_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "voltage" => Some(LtspiceSymbolSpec {
            prefix: "V".to_string(),
            pins: SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "current" => Some(LtspiceSymbolSpec {
            prefix: "I".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "diode" | "led" | "schottky" | "tvsdiode" | "varactor" | "zener" => {
            Some(LtspiceSymbolSpec {
                prefix: "D".to_string(),
                pins: DIODE_PINS.to_vec(),
                source: LtspiceSymbolSpecSource::Builtin,
            })
        }
        "npn" | "npn2" | "npn3" | "npn4" => Some(LtspiceSymbolSpec {
            prefix: "Q".to_string(),
            pins: BJT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "pnp" | "pnp2" | "pnp4" | "lpnp" => Some(LtspiceSymbolSpec {
            prefix: "Q".to_string(),
            pins: BJT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "nmos" | "pmos" => Some(LtspiceSymbolSpec {
            prefix: "M".to_string(),
            pins: MOS_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "nmos4" | "pmos4" => Some(LtspiceSymbolSpec {
            prefix: "M".to_string(),
            pins: MOS4_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "njf" | "pjf" => Some(LtspiceSymbolSpec {
            prefix: "J".to_string(),
            pins: JFET_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "e" | "e2" => Some(LtspiceSymbolSpec {
            prefix: "E".to_string(),
            pins: E_SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "g" | "g2" => Some(LtspiceSymbolSpec {
            prefix: "G".to_string(),
            pins: G_SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "f" => Some(LtspiceSymbolSpec {
            prefix: "F".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "h" => Some(LtspiceSymbolSpec {
            prefix: "H".to_string(),
            pins: SOURCE_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "sw" => Some(LtspiceSymbolSpec {
            prefix: "S".to_string(),
            pins: SWITCH_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "csw" => Some(LtspiceSymbolSpec {
            prefix: "W".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        "bi" | "bv" => Some(LtspiceSymbolSpec {
            prefix: "B".to_string(),
            pins: CURRENT_PINS.to_vec(),
            source: LtspiceSymbolSpecSource::Builtin,
        }),
        _ => None,
    }
}

