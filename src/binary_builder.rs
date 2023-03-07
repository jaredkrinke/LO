use crate::wasm_module::{Instr, WasmModule};
use alloc::vec::Vec;

const FUNC_TYPE: u8 = 0x60;

const SECTION_TYPE: u8 = 0x01;
const SECTION_FUNC: u8 = 0x03;
const SECTION_MEMORY: u8 = 0x05;
const SECTION_EXPORT: u8 = 0x07;
const SECTION_CODE: u8 = 0x0a;

const EXPR_END_OPCODE: u8 = 0x0b;

pub struct BinaryBuilder<'a> {
    module: &'a WasmModule,
    data: Vec<u8>,
}

// TODO(optimize): Where temporary section buffer is needed one buffer can be shared
impl<'a> BinaryBuilder<'a> {
    pub fn new(module: &'a WasmModule) -> Self {
        let data = Vec::new();
        Self { module, data }
    }

    pub fn build(mut self) -> Vec<u8> {
        self.emit_magic_and_version();
        self.emit_type_section();
        self.emit_func_section();
        self.emit_memory_section();
        self.emit_export_section();
        self.emit_code_section();
        self.data
    }

    fn emit_magic_and_version(&mut self) {
        // wasm magic number
        self.data.extend_from_slice(b"\0asm");

        // version
        self.data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
    }

    fn emit_type_section(&mut self) {
        self.data.push(SECTION_TYPE);

        let mut type_section = Vec::new();

        {
            write_u32(&mut type_section, self.module.fn_types.len() as u32);
            for fn_type in &self.module.fn_types {
                type_section.push(FUNC_TYPE);

                write_u32(&mut type_section, fn_type.inputs.len() as u32);
                for &fn_input in &fn_type.inputs {
                    type_section.push(fn_input as u8);
                }

                write_u32(&mut type_section, fn_type.outputs.len() as u32);
                for &fn_output in &fn_type.outputs {
                    type_section.push(fn_output as u8);
                }
            }
        }

        write_u32(&mut self.data, type_section.len() as u32);
        self.data.append(&mut type_section);
    }

    /**
    Currently functions and their types map 1 to 1.

    TODO(optimize): Functions with equivalent types can point to the same type
    */
    fn emit_func_section(&mut self) {
        self.data.push(SECTION_FUNC);

        write_u32(&mut self.data, (self.module.fn_types.len() + 1) as u32);

        write_u32(&mut self.data, self.module.fn_types.len() as u32);
        for i in 0..self.module.fn_types.len() {
            write_u32(&mut self.data, i as u32);
        }
    }

    fn emit_memory_section(&mut self) {
        self.data.push(SECTION_MEMORY);

        let mut memory_section = Vec::new();

        {
            write_u32(&mut memory_section, self.module.memories.len() as u32);
            for memory in &self.module.memories {
                if let Some(memory_max) = memory.max {
                    memory_section.push(0x01);
                    write_u32(&mut memory_section, memory.min as u32);
                    write_u32(&mut memory_section, memory_max as u32);
                } else {
                    memory_section.push(0x00);
                    write_u32(&mut memory_section, memory.min as u32);
                }
            }
        }

        write_u32(&mut self.data, memory_section.len() as u32);
        self.data.append(&mut memory_section);
    }

    fn emit_export_section(&mut self) {
        self.data.push(SECTION_EXPORT);

        let mut export_section = Vec::new();

        {
            write_u32(&mut export_section, self.module.exports.len() as u32);
            for export in &self.module.exports {
                write_u32(&mut export_section, export.export_name.len() as u32);
                export_section.extend_from_slice(export.export_name.as_bytes());

                export_section.push(export.export_type as u8);

                write_u32(&mut export_section, export.exported_item_index as u32);
            }
        }

        write_u32(&mut self.data, export_section.len() as u32);
        self.data.append(&mut export_section);
    }

    fn emit_code_section(&mut self) {
        self.data.push(SECTION_CODE);

        let mut code_section = Vec::new();

        {
            let mut fn_section = Vec::new();

            write_u32(&mut code_section, self.module.fn_codes.len() as u32);
            for fn_code in &self.module.fn_codes {
                {
                    write_u32(&mut fn_section, fn_code.locals.len() as u32);
                    for locals_of_some_type in &fn_code.locals {
                        write_u32(&mut fn_section, locals_of_some_type.count as u32);
                        fn_section.push(locals_of_some_type.value_type as u8);
                    }

                    for instr in &fn_code.expr.instrs {
                        write_instr(&mut fn_section, instr);
                    }

                    fn_section.push(EXPR_END_OPCODE);
                }

                write_u32(&mut code_section, fn_section.len() as u32);
                code_section.append(&mut fn_section);
            }
        }

        write_u32(&mut self.data, code_section.len() as u32);
        self.data.append(&mut code_section);
    }
}

fn write_instr(output: &mut Vec<u8>, instr: &Instr) {
    match instr {
        Instr::I32LessThenSigned { lhs, rhs } => {
            write_instr(output, lhs);
            write_instr(output, rhs);
            output.push(0x48);
        }
        Instr::I32GreaterEqualSigned { lhs, rhs } => {
            write_instr(output, lhs);
            write_instr(output, rhs);
            output.push(0x4e);
        }
        Instr::I32Add { lhs, rhs } => {
            write_instr(output, lhs);
            write_instr(output, rhs);
            output.push(0x6a);
        }
        Instr::I32Sub { lhs, rhs } => {
            write_instr(output, lhs);
            write_instr(output, rhs);
            output.push(0x6b);
        }
        Instr::I32Mul { lhs, rhs } => {
            write_instr(output, lhs);
            write_instr(output, rhs);
            output.push(0x6c);
        }
        Instr::I32Load {
            align,
            offset,
            address_instr,
        } => {
            write_instr(output, address_instr);
            output.push(0x28);
            write_u32(output, *align);
            write_u32(output, *offset);
        }
        Instr::I32Load8Unsigned {
            align,
            offset,
            address_instr,
        } => {
            write_instr(output, address_instr);
            output.push(0x2d);
            write_u32(output, *align);
            write_u32(output, *offset);
        }
        Instr::I32Const(value) => {
            output.push(0x41);
            write_i32(output, *value);
        }
        Instr::LocalGet(local_idx) => {
            output.push(0x20);
            write_u32(output, *local_idx);
        }
        Instr::Return { values } => {
            for value in values {
                write_instr(output, value);
            }
            output.push(0x0f);
        }
        Instr::Call { fn_idx, args } => {
            for arg in args {
                write_instr(output, arg);
            }
            output.push(0x10);
            write_u32(output, *fn_idx);
        }
        Instr::If {
            block_type,
            cond,
            then_branch,
            else_branch,
        } => {
            write_instr(output, cond);
            output.push(0x04); // if
            output.push((*block_type) as u8);
            write_instr(output, then_branch);
            output.push(0x05); // then
            write_instr(output, else_branch);
            output.push(0x0b); // end
        }
        Instr::IfSingleBranch { cond, then_branch } => {
            write_instr(output, cond);
            output.push(0x04); // if
            output.push(0x40); // no value
            write_instr(output, then_branch);
            output.push(0x0b); // end
        }
    }
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    mini_leb128::write_u32(output, value).unwrap();
}

fn write_i32(output: &mut Vec<u8>, value: i32) {
    mini_leb128::write_i32(output, value).unwrap();
}