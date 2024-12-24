use std::fs::File;
use std::io::{self, BufReader, Read};
use std::env;

#[derive(Debug)]
struct ArgsRType {
    rs1: u8,
    rs2: u8,
    rd: u8,
}

#[derive(Debug)]
struct ArgsIType {
    rs1: u8,
    rd: u8,
    imm: i32,
}

#[derive(Debug)]
struct ArgsSBType {
    rs1: u8,
    rs2: u8,
    imm: i32,
}

#[derive(Debug)]
struct ArgsUJType {
    rd: u8,
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
    Slli    (ArgsIType), // shamt
    Srli    (ArgsIType), // shamt
    Srai    (ArgsIType), // shamt
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
}

#[derive(Debug)]
struct IllegalInstruction;

fn decode(instruction: u32) -> Result<Instruction, IllegalInstruction> {
    let opcode = instruction & 0b111_1111;
    let funct3 = (instruction >> 12) & 0b111;
    let funct7 = (instruction >> 25) & 0b111_1111;
    let rs1: u8 = ((instruction >> 15) & 0b1_1111).try_into().unwrap();
    let rs2: u8 = ((instruction >> 20) & 0b1_1111).try_into().unwrap();
    let rd: u8 = ((instruction >> 7) & 0b1_1111).try_into().unwrap();
    let imm = 0xFACADE;

    let args_r = ArgsRType{rs1, rs2, rd};
    let args_i = ArgsIType{rs1, rd, imm};
    let args_sb = ArgsSBType{rs1, rs2, imm};
    let args_uj = ArgsUJType{rd, imm};

    match opcode {
        0b011_0111 => Ok(Instruction::Lui(args_uj)),
        0b001_0111 => Ok(Instruction::Auipc(args_uj)),
        0b110_1111 => Ok(Instruction::Jal(args_uj)),
        0b110_0111 => Ok(Instruction::Jalr(args_i)),
        0b110_0011 => Ok(Instruction::Beq(args_sb)),
        0b000_0011 => Ok(Instruction::Lb(args_i)),
        0b010_0011 => Ok(Instruction::Sb(args_sb)),
        0b001_0011 => Ok(Instruction::Addi(args_i)),
        0b011_0011 => Ok(Instruction::Add(args_r)),
        0b000_1111 => Ok(Instruction::Fence),
        0b111_0011 => Ok(Instruction::Ecall),
        _ => Err(IllegalInstruction)
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];

    println!("path: {:?}", env::current_dir().unwrap());

    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    let mut buffer: [u8; 4] = [0; 4];

    while let Ok(()) = reader.read_exact(&mut buffer) {
        let word = u32::from_le_bytes(buffer);
        println!("{:?}", decode(word).unwrap());
    }
    Ok(())
}
