use ivybridge_rs::display;
use ivybridge_rs::edid::{self, Edid};
use ivybridge_rs::limits::Ceiling;
use ivybridge_rs::report::{self, Input, DEFAULT_MODES};
use ivybridge_rs::timings::{self, VideoMode};
use std::process::ExitCode;

const USAGE: &str = "\
IvyBridge-RS Probe — diagnóstico de modos de vídeo (somente leitura)

Uso: ivyprobe [opções]

  --edid <arquivo>     Lê o EDID de um arquivo (binário ou hex) em vez do sistema
  --dump-edid <arq>    Salva o EDID do monitor ativo em <arq> (hex)
  --mode <WxH@Hz>      Acrescenta um modo à análise (pode repetir)
  --only-modes         Analisa só os modos passados com --mode
  --ceiling <MHz>      Acrescenta um teto de pixel clock (ex.: medido na placa)
  --no-system          Não consulta o Windows
  --add-mode <WxH@Hz>  Gera um DTD CVT-RB e o insere no EDID (pode repetir)
  --write-edid <arq>   Grava o EDID resultante (com --add-mode) em <arq> (hex);
                       nada é aplicado ao sistema
  -h, --help           Mostra esta ajuda
";

struct Args {
    edid_file: Option<String>,
    dump_edid: Option<String>,
    add_modes: Vec<VideoMode>,
    write_edid: Option<String>,
    modes: Vec<VideoMode>,
    only_modes: bool,
    ceilings: Vec<u32>,
    no_system: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        edid_file: None,
        dump_edid: None,
        add_modes: Vec::new(),
        write_edid: None,
        modes: Vec::new(),
        only_modes: false,
        ceilings: Vec::new(),
        no_system: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} precisa de um valor"));
        match arg.as_str() {
            "--edid" => a.edid_file = Some(value("--edid")?),
            "--dump-edid" => a.dump_edid = Some(value("--dump-edid")?),
            "--mode" => {
                let v = value("--mode")?;
                a.modes
                    .push(VideoMode::parse(&v).ok_or(format!("modo inválido: {v}"))?);
            }
            "--add-mode" => {
                let v = value("--add-mode")?;
                a.add_modes
                    .push(VideoMode::parse(&v).ok_or(format!("modo inválido: {v}"))?);
            }
            "--write-edid" => a.write_edid = Some(value("--write-edid")?),
            "--only-modes" => a.only_modes = true,
            "--ceiling" => {
                let v = value("--ceiling")?;
                let mhz: f64 = v.parse().map_err(|_| format!("teto inválido: {v}"))?;
                a.ceilings.push((mhz * 1000.0).round() as u32);
            }
            "--no-system" => a.no_system = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("opção desconhecida: {other}\n\n{USAGE}")),
        }
    }
    Ok(a)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .map(|c| {
            c.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    let system = if args.no_system {
        Err("desativada com --no-system".to_string())
    } else {
        display::query()
    };

    // EDID: arquivo explícito tem prioridade; senão, o da primeira saída ativa.
    let (raw, edid_source) = match &args.edid_file {
        Some(path) => match std::fs::read(path) {
            Ok(b) => (Some(b), format!("arquivo {path}")),
            Err(e) => {
                eprintln!("não foi possível ler {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => match system.as_ref().ok().and_then(|s| {
            s.outputs
                .iter()
                .find_map(|o| o.edid.clone().map(|e| (e, o.monitor_device_path.clone())))
        }) {
            Some((e, path)) => (Some(e), format!("registro do Windows ({path})")),
            None => (None, "nenhuma".into()),
        },
    };

    let (edid, edid_error, bytes) = match raw.map(|r| edid::load_bytes(&r)) {
        Some(Ok(bytes)) => match Edid::parse(&bytes) {
            Ok(e) => (Some(e), None, Some(bytes)),
            Err(err) => (None, Some(err.to_string()), Some(bytes)),
        },
        Some(Err(err)) => (None, Some(err.to_string()), None),
        None => (None, None, None),
    };

    if let (Some(path), Some(bytes)) = (&args.dump_edid, &bytes) {
        if let Err(e) = std::fs::write(path, hex(bytes) + "\n") {
            eprintln!("não foi possível gravar {path}: {e}");
            return ExitCode::FAILURE;
        }
        eprintln!("EDID ({} bytes) salvo em {path}", bytes.len());
    }

    if !args.add_modes.is_empty() {
        let (Some(base), Some(parsed)) = (&bytes, &edid) else {
            eprintln!("--add-mode precisa de um EDID válido");
            return ExitCode::FAILURE;
        };
        let size_mm = (parsed.size_cm.0 as u16 * 10, parsed.size_cm.1 as u16 * 10);
        let mut out = base.clone();
        for m in &args.add_modes {
            let t = timings::cvt_rb(*m);
            let Some(dtd) = edid::encode_dtd(&t, size_mm) else {
                eprintln!("{m}: timing não cabe num DTD");
                return ExitCode::FAILURE;
            };
            match edid::add_dtd_to_cta(&out, &dtd) {
                Ok(o) => out = o,
                Err(e) => {
                    eprintln!("{m}: {e}");
                    return ExitCode::FAILURE;
                }
            }
            eprintln!("+ {t}");
        }
        if let Err(e) = Edid::parse(&out) {
            eprintln!("EDID gerado não passou na validação: {e}");
            return ExitCode::FAILURE;
        }
        match &args.write_edid {
            Some(path) => {
                if let Err(e) = std::fs::write(path, hex(&out) + "\n") {
                    eprintln!("não foi possível gravar {path}: {e}");
                    return ExitCode::FAILURE;
                }
                eprintln!("EDID modificado ({} bytes) salvo em {path}", out.len());
            }
            None => println!("{}", hex(&out)),
        }
        return ExitCode::SUCCESS;
    }

    let mut modes = if args.only_modes {
        Vec::new()
    } else {
        DEFAULT_MODES.to_vec()
    };
    for m in args.modes {
        if !modes.contains(&m) {
            modes.push(m);
        }
    }

    let input = Input {
        system,
        edid,
        edid_source,
        edid_error,
        modes,
        extra_ceilings: args
            .ceilings
            .into_iter()
            .map(|khz| Ceiling {
                label: "informado (--ceiling)".into(),
                khz,
                source: "usuário",
            })
            .collect(),
    };
    print!("{}", report::render(&input));
    ExitCode::SUCCESS
}
