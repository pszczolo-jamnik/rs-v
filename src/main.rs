use std::fs;
use std::fmt::{Display, Formatter};

use elf::abi;
use elf::endian::AnyEndian;
use elf::ElfBytes;

#[derive(Debug)]
struct ArgsRType {
    rs1: usize,
    rs2: usize,
    rd: usize,
}

#[derive(Debug)]
struct ArgsIType {
    rs1: usize,
    rd: usize,
    imm: i32,
    shamt: u8,
    csr: u16,
}

#[derive(Debug)]
struct ArgsSBType {
    rs1: usize,
    rs2: usize,
    imm: i32,
}

#[derive(Debug)]
struct ArgsUJType {
    rd: usize,
    imm: i32,
}

#[derive(Debug)]
enum Instruction {
    Lui     (ArgsUJType),
    Auipc   (ArgsUJType),
    Jal     (ArgsUJType),
    Jalr    (ArgsIType),
    Beq     (ArgsSBType),
    Bne     (ArgsSBType),
    Blt     (ArgsSBType),
    Bge     (ArgsSBType),
    Bltu    (ArgsSBType),
    Bgeu    (ArgsSBType),
    Lb      (ArgsIType),
    Lh      (ArgsIType),
    Lw      (ArgsIType),
    Lbu     (ArgsIType),
    Lhu     (ArgsIType),
    Sb      (ArgsSBType),
    Sh      (ArgsSBType),
    Sw      (ArgsSBType),
    Addi    (ArgsIType),
    Slti    (ArgsIType),
    Sltiu   (ArgsIType),
    Xori    (ArgsIType),
    Ori     (ArgsIType),
    Andi    (ArgsIType),
    Slli    (ArgsIType),
    Srli    (ArgsIType),
    Srai    (ArgsIType),
    Add     (ArgsRType),
    Sub     (ArgsRType),
    Sll     (ArgsRType),
    Slt     (ArgsRType),
    Sltu    (ArgsRType),
    Xor     (ArgsRType),
    Srl     (ArgsRType),
    Sra     (ArgsRType),
    Or      (ArgsRType),
    And     (ArgsRType),
    Fence, // args
    FenceTso,
    Pause,
    Ecall,
    Ebreak,
    Mret,
    Wfi,
    Csrrw   (ArgsIType),
    Csrrs   (ArgsIType),
    Csrrc   (ArgsIType),
    Csrrwi  (ArgsIType),
    Csrrsi  (ArgsIType),
    Csrrci  (ArgsIType),
}

#[derive(Debug)]
struct IllegalInstruction;

#[derive(Debug)]
enum Csr {
    MIsa,
    MVendorId,
    MArchId,
    MImpId,
    MHartId,
    MStatus,
    MIe,
    MTvec,
    MScratch,
    MEpc,
    MCause,
    MTVal,
    MIp,
    MConfigPtr,
}


enum Cause {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAmoAddressMisaligned,
    StoreAmoAccessFault,
    // Ucall,
    // Scall,
    Mcall,
    SoftwareCheck,
    HardwareError,
}

impl Csr {
    fn get_csr(address: u16) -> Option<Self> {
        match address {
            0xF11 => Some(Self::MVendorId),
            0xF12 => Some(Self::MArchId),
            0xF13 => Some(Self::MImpId),
            0xF14 => Some(Self::MHartId),
            0xF15 => Some(Self::MConfigPtr),
            0x300 => Some(Self::MStatus),
            0x301 => Some(Self::MIsa),
            0x304 => Some(Self::MIe),
            0x305 => Some(Self::MTvec),
            0x340 => Some(Self::MScratch),
            0x341 => Some(Self::MEpc),
            0x342 => Some(Self::MCause),
            0x343 => Some(Self::MTVal),
            0x344 => Some(Self::MIp),
            _ => None
        }
    }
}

const MEMORY_SIZE: usize = 4096;

struct CoreState {
    pc: u32,
    regs: [u32; 32],
    memory: [u8; MEMORY_SIZE],
    // M-mode
    mie: bool,
    mpie: bool,
    mtvec: u32,
    mscratch: u32,
    mepc: u32,
    mcause: Cause,
    mtval: u32,
}

impl Display for CoreState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "pc: 0x{:08x}", self.pc)?;
        // for (i, reg) in self.regs.iter().enumerate() {
        //     let new_line = {if i % 4 == 3 {'\n'} else {' '}};
        //     write!(f, "{:>5}: 0x{:08x}{}", Self::reg_name(i), reg, new_line)?;
        // }
        // for m in self.memory {
        //     write!(f, "{:02x} ", m)?;
        // }
        Ok(())
    }
}

impl CoreState {
    fn reg_name(index: usize) -> String {
        match index {
            0 => "zero".to_string(),
            1 => "ra".to_string(),
            2 => "sp".to_string(),
            3 => "gp".to_string(),
            4 => "tp".to_string(),
            5..=7 => format!("t{}", index - 5),
            8..=9 => format!("s{}", index - 8),
            10..=17 => format!("a{}", index - 10),
            18..=27 => format!("s{}", index - 16),
            28..=31 => format!("t{}", index - 25),
            _ => unimplemented!(),
        }
    }

    fn reset(&mut self) {
        self.pc = 0;
        self.mie = false;
        self.mpie = false;
    }

    fn get_csr_value(&self, csr: &Csr) -> u32 {
        match csr {
            // RV32IM
            Csr::MIsa => (1 << 30) | (1 << 8) | (1 << 12),
            Csr::MVendorId => 0,
            Csr::MArchId => 0,
            Csr::MImpId => 0,
            Csr::MHartId => 0,
            Csr::MStatus => (3 << 11) |
                            ((self.mie as u32) << 3) |
                            ((self.mpie as u32) << 7),
            Csr::MIe => 0,
            Csr::MTvec => self.mtvec,
            Csr::MScratch => self.mscratch,
            Csr::MEpc => self.mepc,
            Csr::MCause => Self::get_cause_value(&self.mcause),
            Csr::MTVal => self.mtval,
            Csr::MIp => 0,
            Csr::MConfigPtr => 0,
        }
    }

    fn set_csr_value(&mut self, csr: &Csr, value: u32) {
        match csr {
            Csr::MStatus => {
                self.mie = (value >> 3) & 1 != 0;
                self.mpie = (value >> 7) & 1 != 0;
            }
            Csr::MTvec => self.mtvec = value,
            Csr::MScratch => self.mscratch = value,
            Csr::MEpc => self.mepc = value,
            // Csr::MCause => Self::get_cause_value(&self.mcause),
            Csr::MTVal => self.mtval = value,
            _ => {},
        }
    }

    fn get_cause_value(cause: &Cause) -> u32 {
        match cause {
            Cause::InstructionAddressMisaligned => 0,
            Cause::InstructionAccessFault => 1,
            Cause::IllegalInstruction => 2,
            Cause::Breakpoint => 3,
            Cause::LoadAddressMisaligned => 4,
            Cause::LoadAccessFault => 5,
            Cause::StoreAmoAddressMisaligned => 6,
            Cause::StoreAmoAccessFault => 7,
            Cause::Mcall => 11,
            Cause::SoftwareCheck => 18,
            Cause::HardwareError => 19,
        }
    }

    fn decode(instruction: u32) -> Result<Instruction, IllegalInstruction> {
        let opcode = instruction & 0b111_1111;
        let funct3 = (instruction >> 12) & 0b111;
        let funct7 = (instruction >> 25) & 0b111_1111;

        let rs1: usize = ((instruction >> 15) & 0b1_1111).try_into().unwrap();
        let rs2: usize = ((instruction >> 20) & 0b1_1111).try_into().unwrap();
        let rd: usize = ((instruction >> 7) & 0b1_1111).try_into().unwrap();
        let shamt = rs2 as u8;
        let csr: u16 = ((instruction >> 20) & 0xFFF).try_into().unwrap();

        let imm_i = ((instruction & 0xFFF00000) as i32) >> 20;

        let imm_s = {
            let imm_11_5 = (instruction & 0xFE000000) as i32;
            let imm_4_0 = ((instruction >> 7) & 0x1F) as i32;
            (imm_11_5 >> 20) | imm_4_0
        };

        let imm_b = {
            let imm_12 = (((instruction & 0x80000000) as i32) >> 19) as u32;
            let imm_11 = (instruction & 0x00000080) << 4;
            let imm_10_5 = (instruction >> 20) & 0x7E0;
            let imm_4_1 = (instruction >> 7) & 0x1E;
            (imm_12 | imm_11 | imm_10_5 | imm_4_1) as i32
        };

        let imm_u = (instruction & 0xFFFFF000) as i32;

        let imm_j = {
            let imm_20 = (((instruction & 0x80000000) as i32) >> 11) as u32;
            let imm_19_12 = instruction & 0x000FF000;
            let imm_11 = (instruction & 0x00100000) >> 9;
            let imm_10_1 = (instruction & 0x7FE00000) >> 20;
            (imm_20 | imm_19_12 | imm_11 | imm_10_1) as i32
        };

        let args_r = ArgsRType{rs1, rs2, rd};
        let args_i = ArgsIType{rs1, rd, imm: imm_i, shamt, csr};
        let args_s = ArgsSBType{rs1, rs2, imm: imm_s};
        let args_b = ArgsSBType{rs1, rs2, imm: imm_b};
        let args_u = ArgsUJType{rd, imm: imm_u};
        let args_j = ArgsUJType{rd, imm: imm_j};

        match opcode {
            0b011_0111 => Ok(Instruction::Lui(args_u)),
            0b001_0111 => Ok(Instruction::Auipc(args_u)),
            0b110_1111 => Ok(Instruction::Jal(args_j)),
            0b110_0111 => match funct3 {
                0 => Ok(Instruction::Jalr(args_i)),
                _ => Err(IllegalInstruction),
            }
            0b110_0011 => match funct3 {
                0b000 => Ok(Instruction::Beq(args_b)),
                0b001 => Ok(Instruction::Bne(args_b)),
                0b100 => Ok(Instruction::Blt(args_b)),
                0b101 => Ok(Instruction::Bge(args_b)),
                0b110 => Ok(Instruction::Bltu(args_b)),
                0b111 => Ok(Instruction::Bgeu(args_b)),
                _ => Err(IllegalInstruction),
            }
            0b000_0011 => match funct3 {
                0b000 => Ok(Instruction::Lb(args_i)),
                0b001 => Ok(Instruction::Lh(args_i)),
                0b010 => Ok(Instruction::Lw(args_i)),
                0b100 => Ok(Instruction::Lbu(args_i)),
                0b101 => Ok(Instruction::Lhu(args_i)),
                _ => Err(IllegalInstruction),
            }
            0b010_0011 => match funct3 {
                0b000 => Ok(Instruction::Sb(args_s)),
                0b001 => Ok(Instruction::Sh(args_s)),
                0b010 => Ok(Instruction::Sw(args_s)),
                _ => Err(IllegalInstruction),
            }
            0b001_0011 => match funct3 {
                0b000 => Ok(Instruction::Addi(args_i)),
                0b010 => Ok(Instruction::Slti(args_i)),
                0b011 => Ok(Instruction::Sltiu(args_i)),
                0b100 => Ok(Instruction::Xori(args_i)),
                0b110 => Ok(Instruction::Ori(args_i)),
                0b111 => Ok(Instruction::Andi(args_i)),
                0b001 => match funct7 {
                    0 => Ok(Instruction::Slli(args_i)),
                    _ => Err(IllegalInstruction),
                }
                0b101 => match funct7 {
                    0 => Ok(Instruction::Srli(args_i)),
                    0b010_0000 => Ok(Instruction::Srai(args_i)),
                    _ => Err(IllegalInstruction),
                }
                _ => Err(IllegalInstruction),
            }
            0b011_0011 => match funct7 {
                0 => match funct3 {
                    0b000 => Ok(Instruction::Add(args_r)),
                    0b001 => Ok(Instruction::Sll(args_r)),
                    0b010 => Ok(Instruction::Slt(args_r)),
                    0b011 => Ok(Instruction::Sltu(args_r)),
                    0b100 => Ok(Instruction::Xor(args_r)),
                    0b101 => Ok(Instruction::Srl(args_r)),
                    0b110 => Ok(Instruction::Or(args_r)),
                    0b111 => Ok(Instruction::And(args_r)),
                    _ => Err(IllegalInstruction),
                }
                0b010_0000 => match funct3 {
                    0b000 => Ok(Instruction::Sub(args_r)),
                    0b101 => Ok(Instruction::Sra(args_r)),
                    _ => Err(IllegalInstruction),
                }
                _ => Err(IllegalInstruction),
            }
            0b000_1111 => Ok(Instruction::Fence),
            0b111_0011 => match (funct7, rs2, rs1, funct3, rd) {
                (0, 0, 0, 0, 0) => Ok(Instruction::Ecall),
                (0, 1, 0, 0, 0) => Ok(Instruction::Ebreak),
                (0b001_1000, 0b0_0010, 0, 0, 0) => Ok(Instruction::Mret),
                (0b000_1000, 0b0_0101, 0, 0, 0) => Ok(Instruction::Wfi),
                (_, _, _, 0b001, _) => Ok(Instruction::Csrrw(args_i)),
                (_, _, _, 0b010, _) => Ok(Instruction::Csrrs(args_i)),
                (_, _, _, 0b011, _) => Ok(Instruction::Csrrc(args_i)),
                (_, _, _, 0b101, _) => Ok(Instruction::Csrrwi(args_i)),
                (_, _, _, 0b110, _) => Ok(Instruction::Csrrsi(args_i)),
                (_, _, _, 0b111, _) => Ok(Instruction::Csrrci(args_i)),
                _ => Err(IllegalInstruction),
            }
            _ => Err(IllegalInstruction),
        }
    }

    /// TODO: Refactor branch load store sections
    ///
    /// TODO: Fix rs/rd races
    ///
    fn execute(&mut self) {
        let address = (self.pc as usize)..=((self.pc + 3) as usize);
        let instruction = u32::from_le_bytes(self.memory[address].try_into().expect("fetch error"));
        let instruction = Self::decode(instruction);

        if let Ok(instr) = instruction {

            let jump_branch: bool = match &instr {
                Instruction::Jal(_) |
                Instruction::Jalr(_) |
                Instruction::Beq(_) |
                Instruction::Bne(_) |
                Instruction::Blt(_) |
                Instruction::Bge(_) |
                Instruction::Bltu(_) |
                Instruction::Bgeu(_) => true,
                _ => false
            };

            let mut exception = false;

            match instr {
                Instruction::Lui(args) => {
                    self.regs[args.rd] = args.imm as u32;
                }
                Instruction::Auipc(args) => {
                    self.regs[args.rd] = args.imm as u32 + self.pc;
                }
                Instruction::Jal(args) => {
                    self.regs[args.rd] = self.pc + 4;
                    self.pc += args.imm as u32;
                }
                Instruction::Jalr(args) => {
                    let rs1 = self.regs[args.rs1];
                    self.regs[args.rd] = self.pc + 4;
                    self.pc = (rs1 + (args.imm as u32)) & 0xFFFF_FFFE;
                }
                Instruction::Beq(args) => {
                    self.pc =
                        if self.regs[args.rs1] == self.regs[args.rs2]
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Bne(args) => {
                    self.pc =
                        if self.regs[args.rs1] != self.regs[args.rs2]
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Blt(args) => {
                    self.pc =
                        if (self.regs[args.rs1] as i32) < (self.regs[args.rs2] as i32)
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Bge(args) => {
                    self.pc =
                        if (self.regs[args.rs1] as i32) >= (self.regs[args.rs2] as i32)
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Bltu(args) => {
                    self.pc =
                        if self.regs[args.rs1] < self.regs[args.rs2]
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Bgeu(args) => {
                    self.pc =
                        if self.regs[args.rs1] >= self.regs[args.rs2]
                            {self.pc + (args.imm as u32)} else {self.pc + 4};
                }
                Instruction::Lb(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    self.regs[args.rd] = self.memory[address] as i32 as u32;
                }
                Instruction::Lh(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let address = address..=address + 1;
                    self.regs[args.rd] = u16::from_le_bytes(self.memory[address]
                                                                .try_into()
                                                                .expect("lh error")) as i32 as u32;
                }
                Instruction::Lw(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let address = address..=address + 3;
                    self.regs[args.rd] = u32::from_le_bytes(self.memory[address]
                                                                .try_into()
                                                                .expect("lw error"));
                }
                Instruction::Lbu(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    self.regs[args.rd] = self.memory[address] as u32;
                }
                Instruction::Lhu(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let address = address..=address + 1;
                    self.regs[args.rd] = u16::from_le_bytes(self.memory[address]
                                                                .try_into()
                                                                .expect("lhu error")) as u32;
                }
                Instruction::Sb(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let bytes = self.regs[args.rs2].to_le_bytes();
                    self.memory[address] = bytes[0];
                }
                Instruction::Sh(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let bytes = self.regs[args.rs2].to_le_bytes();
                    self.memory[address] = bytes[0];
                    self.memory[address + 1] = bytes[1];
                }
                Instruction::Sw(args) => {
                    let address = (self.regs[args.rs1] + args.imm as u32) as usize;
                    let bytes = self.regs[args.rs2].to_le_bytes();
                    self.memory[address] = bytes[0];
                    self.memory[address + 1] = bytes[1];
                    self.memory[address + 2] = bytes[2];
                    self.memory[address + 3] = bytes[3];
                }
                Instruction::Addi(args) => {

                }
                Instruction::Slti(args) => {

                }
                Instruction::Sltiu(args) => {

                }
                Instruction::Xori(args) => {

                }
                Instruction::Ori(args) => {

                }
                Instruction::Andi(args) => {

                }
                Instruction::Slli(args) => {

                }
                Instruction::Srli(args) => {
                }
                Instruction::Srai(args) => {
                }
                Instruction::Add(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] + self.regs[args.rs2];
                }
                Instruction::Sub(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] - self.regs[args.rs2];
                }
                Instruction::Sll(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] << (self.regs[args.rs2] & 0b1_1111);
                }
                Instruction::Slt(args) => {
                    self.regs[args.rd] =
                        if (self.regs[args.rs1] as i32) < (self.regs[args.rs2] as i32) {1} else {0};
                }
                Instruction::Sltu(args) => {
                    self.regs[args.rd] =
                        if self.regs[args.rs1] < self.regs[args.rs2] {1} else {0};
                }
                Instruction::Xor(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] ^ self.regs[args.rs2];
                }
                Instruction::Srl(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] >> (self.regs[args.rs2] & 0b1_1111);
                }
                Instruction::Sra(args) => {
                    self.regs[args.rd] = ((self.regs[args.rs1] as i32) >> (self.regs[args.rs2] & 0b1_1111)) as u32;
                }
                Instruction::Or(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] | self.regs[args.rs2];
                }
                Instruction::And(args) => {
                    self.regs[args.rd] = self.regs[args.rs1] & self.regs[args.rs2];
                }
                Instruction::Fence => {}
                Instruction::FenceTso => todo!(),
                Instruction::Pause => todo!(),
                Instruction::Ecall => {
                    exception = true;
                    self.mepc = self.pc;
                    self.mcause = Cause::Mcall;
                }
                Instruction::Ebreak => {
                    exception = true;
                    self.mepc = self.pc;
                    self.mcause = Cause::Breakpoint;
                }
                Instruction::Mret => todo!(),
                Instruction::Wfi => todo!(),
                Instruction::Csrrw(args) => {
                    if let Some(csr) = Csr::get_csr(args.csr) {
                        let rs1 = self.regs[args.rs1];
                        self.regs[args.rd] = self.get_csr_value(&csr);
                        self.set_csr_value(&csr, rs1);
                    } else {
                        exception = true;
                        self.mepc = self.pc;
                        self.mcause = Cause::IllegalInstruction;
                    }
                }
                Instruction::Csrrs(args) => {
                    // println!("{:?}", Csr::get_csr(args.csr));
                }
                Instruction::Csrrc(args) => {
                    // println!("{:?}", Csr::get_csr(args.csr));
                }
                Instruction::Csrrwi(args) => {
                    // println!("{:?}", Csr::get_csr(args.csr));
                }
                Instruction::Csrrsi(args) => {
                    // println!("{:?}", Csr::get_csr(args.csr));
                }
                Instruction::Csrrci(args) => {
                    // println!("{:?}", Csr::get_csr(args.csr));
                }
            }
            match (jump_branch, exception) {
                (_, true) => {
                    self.pc = self.mtvec;
                    println!("😱 it's a trap!");
                    // remove!
                    todo!();
                }
                (false, false) => self.pc += 4,
                (_, _) => {},
            }
            self.regs[0] = 0;
        } else {
            todo!()
        }
    }
}

fn get_tests(path: &str, filter: &str) -> Vec<String> {
    let dir = fs::read_dir(path).unwrap();
    dir
        .map(|entry| String::from(entry.unwrap().path().to_str().unwrap()))
        .filter(|entry| entry.contains(filter) && !entry.ends_with("dump"))
        .collect()

}


fn main() -> std::io::Result<()> {
    let mut core_state = CoreState {
        pc: 0x0000_0000,
        regs: [0; 32],
        memory: [0; MEMORY_SIZE],
        mie: false,
        mpie: false,
        mtvec: 0,
        mscratch: 0,
        mepc: 0,
        mcause: Cause::HardwareError,
        mtval: 0,
    };

    let tests = get_tests("riscv-tests-elf", "rv32ui");

    for test in tests {

        let file_contents = fs::read(&test)
                                        .expect("file read error");
        let elf = ElfBytes::<AnyEndian>::minimal_parse(&file_contents)
                                                .expect("elf parse error");
        let sections = elf.section_headers().expect("elf parse error");

        for section in sections {
            if (abi::SHF_EXECINSTR as u64) & section.sh_flags != 0 {
                let text = elf.section_data(&section).expect("elf parse error").0;
                core_state.memory[..text.len()].copy_from_slice(text);
            }
        }

        let mut pass_pc: u32 = 0;
        let mut fail_pc: u32 = 0;

        let (sym_tab, str_tab) = elf.symbol_table().unwrap().unwrap();
        for sym in sym_tab.iter() {
            let name = str_tab.get(sym.st_name as usize).unwrap();
            match name {
                "pass" => pass_pc = sym.st_value as u32,
                "fail" => fail_pc = sym.st_value as u32,
                _ => {}
            }
        }
        println!("{}", test);
        println!("pass: 0x{:x} fail: 0x{:x}", pass_pc, fail_pc);

        if (pass_pc == 0) || (fail_pc == 0) {
            println!("🟡");
            continue;
        }

        core_state.reset();

        loop {
            println!("{}", core_state);
            core_state.execute();
            match core_state.pc {
                p if p == pass_pc => {println!("🟢"); break;},
                f if f == fail_pc => {println!("🔴"); break;},
                _ => {}
            }
        }
    }


    Ok(())
}
