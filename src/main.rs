use std::fs::File;
use std::io::{self, BufReader, Read};
use std::env;
use std::fmt::{Display, Formatter};

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

const MEMORY_SIZE: usize = 4096;

struct CoreState {
    pc: u32,
    regs: [u32; 32],
    memory: [u8; MEMORY_SIZE],
}

impl Display for CoreState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "pc: 0x{:08x}", self.pc)?;
        for (i, reg) in self.regs.iter().enumerate() {
            let new_line = {if i % 4 == 3 {'\n'} else {' '}};
            write!(f, "{:>5}: 0x{:08x}{}", Self::reg_name(i), reg, new_line)?;
        }
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

    fn execute(&mut self, instruction: Result<Instruction, IllegalInstruction>) {
        if let Ok(instr) = instruction {
            match instr {
                Instruction::Lui(args) => {
                    self.regs[args.rd] = args.imm as u32;
                }
                Instruction::Auipc(args) => {
                    self.regs[args.rd] = args.imm as u32 + self.pc;
                }
                Instruction::Jalr(args) => {
                    self.regs[args.rd] = self.pc + 4;
                    self.pc = (self.regs[args.rs1] + (args.imm as u32)) & 0xFFFF_FFFE;
                }
                _ => todo!(),
            }
            self.regs[0] = 0;
        } else {
            todo!()
        }
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
        let imm_12 = ((instruction & 0x80000000) as i32) >> 19;
        let imm_11 = ((instruction & 0x00000080) as i32) << 4;
        let imm_10_5 = ((instruction >> 20) & 0x3F) as i32;
        let imm_4_1 = ((instruction >> 7) & 0xF) as i32;
        imm_12 | imm_11 | (imm_10_5 << 1) | (imm_4_1 << 1)
    };

    let imm_u = ((instruction & 0xFFFFF000) as i32);

    let imm_j = {
        let imm_20 = ((instruction & 0x80000000) as i32) >> 11;
        let imm_19_12 = ((instruction & 0x000FF000) as i32);
        let imm_11 = ((instruction & 0x00100000) as i32) >> 9;
        let imm_10_1 = ((instruction & 0x7FE00000) as i32) >> 20;
        imm_20 | imm_19_12 | imm_11 | imm_10_1
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

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];

    println!("path: {:?}", env::current_dir().unwrap());

    let mut core_state = CoreState {
        pc: 0x2000_0000,
        regs: [0; 32],
        memory: [0; MEMORY_SIZE],
    };

    println!("{}", core_state);

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    let mut buffer: [u8; 4] = [0; 4];

    while let Ok(()) = reader.read_exact(&mut buffer) {
        let word = u32::from_le_bytes(buffer);
        let instruction = decode(word);
        println!("{:?}", instruction.as_ref().unwrap());
        core_state.execute(instruction);
        println!("{}", core_state);
    }
    Ok(())
}
