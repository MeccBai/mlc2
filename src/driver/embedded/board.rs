use serde::Deserialize;
use std::{collections::BTreeMap, fmt::Write, fs, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(super) struct Board {
    memory: BTreeMap<String, Memory>,
    placement: Placement,
    #[serde(default)]
    interrupt: BTreeMap<String, String>,
    #[serde(default)]
    copy: Vec<CopyRegion>,
    #[serde(default)]
    zero: Vec<ZeroRegion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Memory {
    start: u32,
    length: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Placement {
    vectors: String,
    code: String,
    stack: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct CopyRegion {
    section: String,
    load: String,
    run: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct ZeroRegion {
    section: String,
    region: String,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
}

impl Board {
    pub(super) fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let board: Self = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        for (name, region) in &board.memory {
            if !identifier(name)
                || region.length == 0
                || region.start.checked_add(region.length).is_none()
            {
                return Err(format!("Invalid board memory region: {name}"));
            }
        }
        board.regions()?;
        Ok(board)
    }

    fn regions(&self) -> Result<(&str, &str, &str), String> {
        let vectors = self.placement.vectors.as_str();
        let code = self.placement.code.as_str();
        let ram = self.placement.stack.as_str();
        for region in [vectors, code, ram] {
            if !self.memory.contains_key(region) {
                return Err(format!("Unknown board memory region: {region}"));
            }
        }
        for copy in &self.copy {
            if copy.section != ".data" || copy.load != code || copy.run != ram {
                return Err(
                    "Board.Copy currently supports only .data from Code to Stack memory".into(),
                );
            }
        }
        for zero in &self.zero {
            if zero.section != ".bss" || zero.region != ram {
                return Err("Board.Zero currently supports only .bss in Stack memory".into());
            }
        }
        if self.copy.len() > 1 || self.zero.len() > 1 {
            return Err("Duplicate board initialization sections".into());
        }
        Ok((vectors, code, ram))
    }

    pub(super) fn linker_script(&self) -> Result<String, String> {
        let (vectors, code, ram) = self.regions()?;
        let mut script = String::from("ENTRY(Reset_Handler)\nMEMORY {\n");
        for (name, region) in &self.memory {
            writeln!(
                script,
                "  {name} : ORIGIN = {:#x}, LENGTH = {:#x}",
                region.start, region.length
            )
            .unwrap();
        }
        writeln!(
            script,
            "}}
_estack = (ORIGIN({ram}) + LENGTH({ram})) & ~7;
SECTIONS {{
  .isr_vector ORIGIN({vectors}) : {{ KEEP(*(.isr_vector)) }} > {vectors}
  .text : {{ *(.text .text.*) *(.rodata .rodata.*) }} > {code}
  .data : {{ _sdata = .; *(.data .data.*); _edata = .; }} > {ram} AT > {code}
  _sidata = LOADADDR(.data);
  .bss (NOLOAD) : {{ _sbss = .; *(.bss .bss.* COMMON); _ebss = .; }} > {ram}
  /DISCARD/ : {{ *(.ARM.exidx* .ARM.extab* .eh_frame*) }}
}}
ASSERT(_ebss < _estack, \"No RAM left for stack\")"
        )
        .unwrap();
        Ok(script)
    }

    pub(super) fn startup(&self, main_return: &str, initializer: &str) -> Result<String, String> {
        use mlc_codegen::gens::instruction::IrValue;
        let initializer = IrValue::Global(initializer.to_owned());
        let mut vectors = vec![None; 16];
        for (slot, name) in [
            (0, "_estack"),
            (1, "Reset_Handler"),
            (2, "NMI_Handler"),
            (3, "HardFault_Handler"),
            (4, "MemManage_Handler"),
            (5, "BusFault_Handler"),
            (6, "UsageFault_Handler"),
            (11, "SVC_Handler"),
            (12, "DebugMon_Handler"),
            (14, "PendSV_Handler"),
            (15, "SysTick_Handler"),
        ] {
            vectors[slot] = Some(name);
        }
        for (irq, name) in &self.interrupt {
            let irq: usize = irq
                .parse()
                .ok()
                .filter(|irq| *irq < 240)
                .ok_or_else(|| format!("Invalid Cortex-M IRQ slot: {irq}"))?;
            if !identifier(name)
                || matches!(
                    name.as_str(),
                    "main" | "Reset_Handler" | "_estack" | "Default_Handler"
                )
            {
                return Err(format!("Invalid board IRQ symbol: {name}"));
            }
            let slot = 16 + irq;
            if slot >= vectors.len() {
                vectors.resize(slot + 1, None);
            }
            if vectors[slot].replace(name).is_some() {
                return Err(format!("Duplicate IRQ slot: {irq}"));
            }
        }
        let mut ir = format!("declare {main_return} @main()\ndeclare void {initializer}()\n");
        for symbol in ["_estack", "_sidata", "_sdata", "_edata", "_sbss", "_ebss"] {
            writeln!(ir, "@{symbol} = external global i8").unwrap();
        }
        let entries = vectors
            .iter()
            .map(|name| match name {
                Some(name) => format!("ptr @{name}"),
                None => "ptr null".into(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            ir,
            "@__mlc_vectors = constant [{} x ptr] [{entries}], section \".isr_vector\", align 4",
            vectors.len()
        )
        .unwrap();
        let mut emitted = std::collections::HashSet::new();
        for name in vectors.into_iter().skip(2).flatten() {
            if emitted.insert(name) {
                writeln!(ir, "@{name} = weak alias void (), ptr @Default_Handler").unwrap();
            }
        }
        ir.push_str(include_str!("reset.ll"));
        writeln!(ir, "  call void {initializer}()\n  call {main_return} @main()\n  br label %halt\nhalt:\n  br label %halt\n}}").unwrap();
        Ok(ir)
    }
}
