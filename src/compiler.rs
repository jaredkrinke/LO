use crate::{
    parser::SExpr,
    wasm_module::{Export, ExportType, Expr, FnCode, FnType, Instr, ValueType, WasmModule},
};
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};

pub fn compile_module(exprs: &Vec<SExpr>) -> Result<WasmModule, String> {
    let mut module = WasmModule::default();

    let mut fn_types = BTreeMap::<String, FnType>::new();
    let mut fn_codes = BTreeMap::<String, FnCode>::new();
    let mut fn_exports = BTreeMap::<String, String>::new();

    for expr in exprs {
        let SExpr::List(items) = expr else {
            return Err(String::from("Unexpected atom"));
        };

        let [SExpr::Atom(op), other @ ..] = &items[..] else {
            return Err(String::from("Expected operation, got a simple list"));
        };

        // TODO: cleanup
        match (op.as_str(), &other[..]) {
            ("::", [SExpr::Atom(name), SExpr::List(inputs), SExpr::List(outputs)]) => {
                if fn_types.contains_key(name) {
                    return Err(format!("Cannot redefine function type: {name}"));
                }

                let mut fn_inputs = vec![];
                for input in inputs {
                    fn_inputs.push(parse_value_type(get_atom_text(input)?)?);
                }

                let mut fn_outputs = vec![];
                for input in outputs {
                    fn_outputs.push(parse_value_type(get_atom_text(input)?)?);
                }

                fn_types.insert(
                    name.clone(),
                    FnType {
                        inputs: fn_inputs,
                        outputs: fn_outputs,
                    },
                );
            }
            ("fn", [SExpr::Atom(name), SExpr::List(params), SExpr::List(instrs)]) => {
                if fn_codes.contains_key(name) {
                    return Err(format!("Cannot redefine function body: {name}"));
                }

                let mut fn_args = BTreeMap::new();
                for (idx, input) in params.iter().enumerate() {
                    let SExpr::Atom(param_name) = input else {
                        return Err(format!("Unexpected list is parameters"));
                    };

                    fn_args.insert(param_name.as_str(), idx as u32);
                }

                fn_codes.insert(
                    name.clone(),
                    FnCode {
                        locals: vec![], // TODO: implement
                        expr: Expr {
                            instrs: parse_instrs(instrs, &fn_args, &fn_types)?,
                        },
                    },
                );
            }
            ("export", [SExpr::Atom(in_name), SExpr::Atom(as_literal), SExpr::Atom(out_name)])
                if as_literal == ":as" =>
            {
                if !fn_types.contains_key(in_name) {
                    return Err(format!("Cannot export {in_name}, export type is unknown"));
                }

                fn_exports.insert(in_name.clone(), out_name.clone());
            }
            (op, _) => return Err(format!("Unknown operation: {op}")),
        };
    }

    let fn_names = fn_types.keys().cloned().collect::<Vec<_>>();

    if !fn_codes.keys().eq(fn_names.iter()) {
        // TODO: better error message?
        return Err(format!("Function types and codes do not match"));
    }

    for (fn_index, fn_name) in fn_names.iter().enumerate() {
        module.fn_types.push(fn_types.remove(fn_name).unwrap());
        module
            .fn_codes
            .push(fn_codes.remove(fn_name).ok_or_else(|| String::from(""))?);

        if let Some(export_name) = fn_exports.remove(fn_name) {
            module.exports.push(Export {
                export_type: ExportType::Func,
                export_name,
                exported_item_index: fn_index,
            });
        }
    }

    Ok(module)
}

fn parse_instrs(
    exprs: &Vec<SExpr>,
    fn_args: &BTreeMap<&str, u32>,
    fn_types: &BTreeMap<String, FnType>,
) -> Result<Vec<Instr>, String> {
    let mut instrs = vec![];

    for expr in exprs {
        let SExpr::List(items) = expr else {
            return Err(format!("Unexpected atom"));
        };

        let [SExpr::Atom(op), args @ ..] = &items[..] else {
            return Err(format!("Expected operation, got a simple list"));
        };

        let instr = match (op.as_str(), &args[..]) {
            ("i32.const", [SExpr::Atom(value)]) => Instr::I32Const(
                value
                    .parse()
                    .map_err(|_| String::from("Parsing i32 failed"))?,
            ),
            ("i32.lt_s", []) => Instr::I32LTS,
            ("i32.sub", []) => Instr::I32Sub,
            ("i32.mul", []) => Instr::I32Mul,
            ("local.get", [SExpr::Atom(local_name)]) => {
                let Some(&idx) = fn_args.get(local_name.as_str()) else {
                    return Err(format!("Unknown location for local.get: {local_name}"));
                };

                Instr::LocalGet(idx)
            }
            (
                "if",
                [SExpr::Atom(block_type), SExpr::List(then_branch), SExpr::List(else_branch)],
            ) => Instr::If(
                parse_value_type(block_type)?,
                parse_instrs(then_branch, fn_args, fn_types)?,
                parse_instrs(else_branch, fn_args, fn_types)?,
            ),
            ("call", [SExpr::Atom(fn_name)]) => {
                let Some(fn_idx) = fn_types.keys().position(|k| k == fn_name) else {
                    return Err(format!("Trying to call unknown function: {fn_name}"));
                };

                Instr::Call(fn_idx as u32)
            }
            ("return", []) => Instr::Return,
            (instr, _) => return Err(format!("Unknown instuction: {instr}")),
        };

        instrs.push(instr);
    }

    Ok(instrs)
}

fn parse_value_type(name: &str) -> Result<ValueType, String> {
    match name {
        "i32" => Ok(ValueType::I32),
        "i64" => Ok(ValueType::I64),
        "f32" => Ok(ValueType::F32),
        "f64" => Ok(ValueType::F64),
        "v128" => Ok(ValueType::V128),
        "funcref" => Ok(ValueType::FuncRef),
        "externref" => Ok(ValueType::ExternRef),
        _ => return Err(format!("Unknown value type: {name}")),
    }
}

fn get_atom_text(expr: &SExpr) -> Result<&str, String> {
    match expr {
        SExpr::Atom(atom_text) => Ok(&atom_text),
        _ => return Err(format!("Atom expected, list found")),
    }
}
