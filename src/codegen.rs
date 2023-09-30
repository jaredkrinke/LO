use crate::wasm::*;
use alloc::vec::Vec;

pub fn codegen(module: &WasmModule) -> Vec<u8> {
    let mut binary = Vec::new();
    let mut section = Vec::new();

    write_magic_and_version(&mut binary);

    write_type_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x01);

    write_import_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x02);

    write_function_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x03);

    write_memory_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x05);

    write_global_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x06);

    write_export_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x07);

    write_code_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x0a);

    write_data_section(&mut section, module);
    write_section(&mut binary, &mut section, 0x0b);

    binary
}

fn write_section(out: &mut Vec<u8>, section: &mut Vec<u8>, section_code: u8) {
    write_u8(out, section_code);
    write_u32(out, section.len() as u32);
    out.append(section);
}

fn write_magic_and_version(out: &mut Vec<u8>) {
    // wasm magic number
    write_all(out, b"\0asm");

    // version
    write_all(out, &[0x01, 0x00, 0x00, 0x00]);
}

fn write_type_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.types.len() as u32);
    for fn_type in &module.types {
        write_u8(out, 0x60); // func type

        write_u32(out, fn_type.inputs.len() as u32);
        for &fn_input in &fn_type.inputs {
            write_u8(out, fn_input as u8);
        }

        write_u32(out, fn_type.outputs.len() as u32);
        for &fn_output in &fn_type.outputs {
            write_u8(out, fn_output as u8);
        }
    }
}

fn write_import_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.imports.len() as u32);
    for import in &module.imports {
        write_u32(out, import.module_name.len() as u32);
        write_all(out, import.module_name.as_bytes());

        write_u32(out, import.item_name.len() as u32);
        write_all(out, import.item_name.as_bytes());

        match import.item_desc {
            WasmImportDesc::Func { type_index } => {
                write_u32(out, 0x00); // fn
                write_u32(out, type_index);
            }
        }
    }
}

fn write_function_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.functions.len() as u32);
    for type_index in &module.functions {
        write_u32(out, *type_index);
    }
}

fn write_memory_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.memories.len() as u32);
    for memory in &module.memories {
        if let Some(memory_max) = memory.max {
            write_u8(out, 0x01);
            write_u32(out, memory.min as u32);
            write_u32(out, memory_max as u32);
        } else {
            write_u8(out, 0x00);
            write_u32(out, memory.min as u32);
        }
    }
}

fn write_global_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.globals.len() as u32);
    for global in &module.globals {
        write_u8(out, global.kind.value_type as u8);

        if global.kind.mutable {
            write_u8(out, 0x01);
        } else {
            write_u8(out, 0x00);
        }

        write_expr(out, &global.initial_value);
    }
}

fn write_export_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.exports.len() as u32);
    for export in &module.exports {
        write_u32(out, export.export_name.len() as u32);
        write_all(out, export.export_name.as_bytes());

        write_u8(out, export.export_type as u8);

        write_u32(out, export.exported_item_index);
    }
}

fn write_code_section(out: &mut Vec<u8>, module: &WasmModule) {
    let mut fn_section = Vec::new();

    write_u32(out, module.codes.len() as u32);
    for fn_code in &module.codes {
        write_u32(&mut fn_section, fn_code.locals.len() as u32);
        for locals_of_some_type in &fn_code.locals {
            write_u32(&mut fn_section, locals_of_some_type.count as u32);
            write_u8(&mut fn_section, locals_of_some_type.value_type as u8);
        }
        write_expr(&mut fn_section, &fn_code.expr);

        write_u32(out, fn_section.len() as u32);
        out.append(&mut fn_section);
    }
}

fn write_data_section(out: &mut Vec<u8>, module: &WasmModule) {
    write_u32(out, module.datas.borrow().len() as u32);
    for data in module.datas.borrow().iter() {
        let WasmData::Active { offset, bytes } = data;
        write_u32(out, 0);
        write_expr(out, offset);
        write_u32(out, bytes.len() as u32);
        write_all(out, bytes);
    }
}

fn write_expr(out: &mut Vec<u8>, expr: &WasmExpr) {
    for instr in &expr.instrs {
        write_instr(out, instr);
    }

    write_u8(out, 0x0b); // end
}

fn write_instr(out: &mut Vec<u8>, instr: &WasmInstr) {
    match instr {
        WasmInstr::Unreachable => {
            write_u8(out, 0x00);
        }
        WasmInstr::BinaryOp { kind } => {
            write_u8(out, *kind as u8);
        }
        WasmInstr::MemorySize => {
            write_u8(out, 0x3F);
            write_u8(out, 0x00);
        }
        WasmInstr::MemoryGrow => {
            write_u8(out, 0x40);
            write_u8(out, 0x00);
        }
        WasmInstr::Load {
            kind,
            align,
            offset,
        } => {
            write_u8(out, *kind as u8);
            write_u32(out, *align);
            write_u32(out, *offset);
        }
        WasmInstr::I32Const { value } => {
            write_u8(out, 0x41);
            write_i32(out, *value);
        }
        WasmInstr::I32ConstLazy { value } => {
            write_u8(out, 0x41);
            write_i32(out, *value.borrow());
        }
        WasmInstr::I64Const { value } => {
            write_u8(out, 0x42);
            write_i64(out, *value);
        }
        WasmInstr::LocalGet { local_index } => {
            write_u8(out, 0x20);
            write_u32(out, *local_index);
        }
        WasmInstr::GlobalGet { global_index } => {
            write_u8(out, 0x23);
            write_u32(out, *global_index);
        }
        WasmInstr::LocalSet { local_index } => {
            write_u8(out, 0x21);
            write_u32(out, *local_index);
        }
        WasmInstr::GlobalSet { global_index } => {
            write_u8(out, 0x24);
            write_u32(out, *global_index);
        }
        WasmInstr::Store {
            align,
            offset,
            kind,
        } => {
            write_u8(out, *kind as u8);
            write_u32(out, *align);
            write_u32(out, *offset);
        }
        WasmInstr::Drop => {
            write_u8(out, 0x1A);
        }
        WasmInstr::Return => {
            write_u8(out, 0x0f);
        }
        WasmInstr::Call { fn_index } => {
            write_u8(out, 0x10);
            write_u32(out, *fn_index);
        }
        WasmInstr::BlockStart {
            block_type,
            return_type,
        } => {
            write_u8(out, (*block_type) as u8);
            if let Some(return_type) = return_type {
                write_u8(out, (*return_type) as u8);
            } else {
                write_u8(out, 0x40); // no value
            }
        }
        WasmInstr::Else => {
            write_u8(out, 0x05);
        }
        WasmInstr::BlockEnd => {
            write_u8(out, 0x0b);
        }
        WasmInstr::Branch { label_index } => {
            write_u8(out, 0x0c);
            write_u32(out, *label_index);
        }
    }
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    mini_leb128::write_u32(out, value).unwrap();
}

fn write_i32(out: &mut Vec<u8>, value: i32) {
    mini_leb128::write_i32(out, value).unwrap();
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    mini_leb128::write_i64(out, value).unwrap();
}

fn write_all(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(value);
}
