import {
  DynamicLibrary,
  ForeignFunction,
  LibraryObject,
  LibraryObjectDefinitionBase,
  LibraryObjectDefinitionInferenceMarker,
  LibraryObjectDefinitionToLibraryDefinition,
} from 'ffi-napi';
import arrayRefLib, { ArrayType, TypedArray } from 'ref-array-di';
import refLib, { allocCString, NULL, Pointer, ref, refType } from 'ref-napi';

const arrayRef = arrayRefLib(refLib);

const stringRef = refType('string');
const nullRef = refType('void');

export type LibLLVM = ReturnType<typeof loadLibLLVMInternal>;
export const loadLibLLVM: (libFile?: string) => LibLLVM = loadLibLLVMInternal;

function loadLibLLVMInternal(libFile = '/usr/lib/llvm-13/lib/libLLVM.so') {
  const fn = wrapLib(DynamicLibrary(libFile));

  return {
    contextCreate: fn({
      name: 'LLVMContextCreate',
      type: [LLVMContext.TYPE, []],
      wrap: (call) => () => new LLVMContext(call()),
    }),
    contextDispose: fn({
      name: 'LLVMContextDispose',
      type: ['void', [LLVMContext.TYPE]],
      wrap: (call) => (ctx: LLVMContext) => call(ctx.value),
    }),

    moduleCreateWithNameInContext: fn({
      name: 'LLVMModuleCreateWithNameInContext',
      type: [LLVMModule.TYPE, ['string', LLVMContext.TYPE]],
      wrap: (call) => (moduleName: string, ctx: LLVMContext) =>
        new LLVMModule(call(moduleName, ctx.value)),
    }),
    disposeModule: fn({
      name: 'LLVMDisposeModule',
      type: ['void', [LLVMModule.TYPE]],
      wrap: (call) => (module: LLVMModule) => call(module.value),
    }),

    createBuilderInContext: fn({
      name: 'LLVMCreateBuilderInContext',
      type: [LLVMIRBuilder.TYPE, [LLVMContext.TYPE]],

      wrap: (call) => (ctx: LLVMContext) => new LLVMIRBuilder(call(ctx.value)),
    }),
    disposeBuilder: fn({
      name: 'LLVMDisposeBuilder',
      type: ['void', [LLVMIRBuilder.TYPE]],
      wrap: (call) => (builder: LLVMIRBuilder) => call(builder.value),
    }),

    setTarget: fn({
      name: 'LLVMSetTarget',
      type: ['void', [LLVMModule.TYPE, 'string']],
      wrap: (call) => (module: LLVMModule, targetTriple: string) =>
        call(module.value, targetTriple),
    }),

    pointerType: fn({
      name: 'LLVMPointerType',
      type: [LLVMType.TYPE, [LLVMType.TYPE, 'int']],
      wrap: (call) => (type: LLVMType) => new LLVMType(call(type.value, 0)),
    }),
    voidTypeInContext: fn({
      name: 'LLVMVoidTypeInContext',
      type: [LLVMType.TYPE, [LLVMContext.TYPE]],
      wrap: (call) => (ctx: LLVMContext) => new LLVMType(call(ctx.value)),
    }),
    i8TypeInContext: fn({
      name: 'LLVMInt8TypeInContext',
      type: [LLVMType.TYPE, [LLVMContext.TYPE]],
      wrap: (call) => (ctx: LLVMContext) => new LLVMType(call(ctx.value)),
    }),
    i32TypeInContext: fn({
      name: 'LLVMInt32TypeInContext',
      type: [LLVMType.TYPE, [LLVMContext.TYPE]],
      wrap: (call) => (ctx: LLVMContext) => new LLVMType(call(ctx.value)),
    }),
    functionType: fn({
      name: 'LLVMFunctionType',
      type: [LLVMType.TYPE, [LLVMType.TYPE, LLVMTypeArray, 'int', 'bool']],
      wrap:
        (call) =>
        (returnType: LLVMType, argTypes: LLVMType[], isVarArg = false) =>
          new LLVMType(
            call(
              returnType.value,
              buildArray(LLVMTypeArray, argTypes),
              argTypes.length,
              isVarArg,
            ),
          ),
    }),
    arrayType: fn({
      name: 'LLVMArrayType',
      type: [LLVMType.TYPE, [LLVMType.TYPE, 'int']],
      wrap: (call) => (type: LLVMType, length: number) =>
        new LLVMType(call(type.value, length)),
    }),

    getUndef: fn({
      name: 'LLVMGetUndef',
      type: [LLVMValue.TYPE, [LLVMType.TYPE]],
      wrap: (call) => (type: LLVMType) => new LLVMValue(call(type.value)),
    }),
    constInt: fn({
      name: 'LLVMConstInt',
      type: [LLVMValue.TYPE, [LLVMType.TYPE, 'int', 'bool']],
      wrap:
        (call) =>
        (type: LLVMType, value: number, signExtend = false) =>
          new LLVMValue(call(type.value, value, signExtend)),
    }),
    constPointerNull: fn({
      name: 'LLVMConstPointerNull',
      type: [LLVMValue.TYPE, [LLVMType.TYPE]],
      wrap: (call) => (type: LLVMType) => new LLVMValue(call(type.value)),
    }),

    addFunction: fn({
      name: 'LLVMAddFunction',
      type: [LLVMValue.TYPE, [LLVMModule.TYPE, 'string', LLVMType.TYPE]],
      wrap: (call) => (module: LLVMModule, fnName: string, type: LLVMType) =>
        new LLVMValue(call(module.value, fnName, type.value)),
    }),
    getNamedFunction: fn({
      name: 'LLVMGetNamedFunction',
      type: [LLVMValue.TYPE, [LLVMModule.TYPE, 'string']],
      wrap: (call) => (module: LLVMModule, fnName: string) =>
        new LLVMValue(call(module.value, fnName)),
    }),

    appendBasicBlockInContext: fn({
      name: 'LLVMAppendBasicBlockInContext',
      type: [LLVMBasicBlock.TYPE, [LLVMContext.TYPE, LLVMValue.TYPE, 'string']],
      wrap: (call) => (ctx: LLVMContext, fn: LLVMValue, name: string) =>
        new LLVMBasicBlock(call(ctx.value, fn.value, name)),
    }),

    positionBuilderAtEnd: fn({
      name: 'LLVMPositionBuilderAtEnd',
      type: ['void', [LLVMIRBuilder.TYPE, LLVMBasicBlock.TYPE]],
      wrap: (call) => (builder: LLVMIRBuilder, block: LLVMBasicBlock) =>
        call(builder.value, block.value),
    }),
    buildGlobalStringPtr: fn({
      name: 'LLVMBuildGlobalStringPtr',
      type: [LLVMValue.TYPE, [LLVMIRBuilder.TYPE, 'string', 'string']],
      wrap:
        (call) =>
        (builder: LLVMIRBuilder, content: string, name = 'str') =>
          new LLVMValue(call(builder.value, content, name)),
    }),
    buildRet: fn({
      name: 'LLVMBuildRet',
      type: [LLVMValue.TYPE, [LLVMIRBuilder.TYPE, LLVMValue.TYPE]],
      wrap: (call) => (builder: LLVMIRBuilder, value: LLVMValue) =>
        new LLVMValue(call(builder.value, value.value)),
    }),
    buildCall: fn({
      name: 'LLVMBuildCall',
      type: [
        LLVMValue.TYPE,
        [LLVMIRBuilder.TYPE, LLVMValue.TYPE, LLVMValueArray, 'int', 'string'],
      ],
      wrap:
        (call) =>
        (builder: LLVMIRBuilder, fn: LLVMValue, args: LLVMValue[], name = '') =>
          new LLVMValue(
            call(
              builder.value,
              fn.value,
              buildArray(LLVMValueArray, args),
              args.length,
              name,
            ),
          ),
    }),
    buildAlloca: fn({
      name: 'LLVMBuildAlloca',
      type: [LLVMValue.TYPE, [LLVMIRBuilder.TYPE, LLVMType.TYPE, 'string']],
      wrap:
        (call) =>
        (builder: LLVMIRBuilder, type: LLVMType, name = '') =>
          new LLVMValue(call(builder.value, type.value, name)),
    }),
    buildStore: fn({
      name: 'LLVMBuildStore',
      type: [
        LLVMValue.TYPE,
        [LLVMIRBuilder.TYPE, LLVMValue.TYPE, LLVMValue.TYPE],
      ],
      wrap:
        (call) =>
        (builder: LLVMIRBuilder, value: LLVMValue, pointer: LLVMValue) =>
          new LLVMValue(call(builder.value, value.value, pointer.value)),
    }),
    buildGEP: fn({
      name: 'LLVMBuildGEP',
      type: [
        LLVMValue.TYPE,
        [LLVMIRBuilder.TYPE, LLVMValue.TYPE, LLVMValueArray, 'int', 'string'],
      ],
      wrap:
        (call) =>
        (
          builder: LLVMIRBuilder,
          pointer: LLVMValue,
          indices: LLVMValue[],
          name = '',
        ) => {
          return new LLVMValue(
            call(
              builder.value,
              pointer.value,
              buildArray(LLVMValueArray, indices),
              indices.length,
              name,
            ),
          );
        },
    }),

    verifyFunction: fn({
      name: 'LLVMVerifyFunction',
      type: ['bool', [LLVMValue.TYPE, 'int']],
      wrap: (call) => (fn: LLVMValue) => ({ ok: !call(fn.value, 2) }),
    }),
    verifyModule: fn({
      name: 'LLVMVerifyModule',
      type: ['bool', [LLVMModule.TYPE, 'int', stringRef]],
      wrap: (call) => (module: LLVMModule) => {
        const messageRef = ref(allocCString(' '.repeat(2048)));
        const err = call(module.value, 2, messageRef);
        const message = messageRef.deref().toString();

        return { ok: !err, message };
      },
    }),

    printModuleToFile: fn({
      name: 'LLVMPrintModuleToFile',
      type: ['void', [LLVMModule.TYPE, 'string', nullRef]],
      wrap: (call) => (module: LLVMModule, fileName: string) =>
        call(module.value, fileName, NULL as never),
    }),
  };
}

type ValueOf<R> = R extends Record<string, infer V> ? V : never;

type FunctionType<
  T extends
    | ValueOf<LibraryObjectDefinitionBase>
    | ValueOf<LibraryObjectDefinitionInferenceMarker>,
> = (
  value: T,
) => LibraryObject<LibraryObjectDefinitionToLibraryDefinition<{ k: T }>>['k'];

type WrapFunctionParams<
  W,
  T extends
    | ValueOf<LibraryObjectDefinitionBase>
    | ValueOf<LibraryObjectDefinitionInferenceMarker>,
> = {
  name: string;
  type: T;
  wrap: (fn: ReturnType<FunctionType<T>>) => W;
};

function wrapLib(lib: DynamicLibrary) {
  return <
    W,
    T extends
      | ValueOf<LibraryObjectDefinitionBase>
      | ValueOf<LibraryObjectDefinitionInferenceMarker>,
  >({
    name,
    type: [returnType, argTypes],
    wrap,
  }: WrapFunctionParams<W, T>) => {
    // TODO: try to not use `as never`
    return wrap(ForeignFunction(lib.get(name), returnType, argTypes) as never);
  };
}

class UniqueType<T> {
  constructor(public readonly value: T) {}
}

export class LLVMContext extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

export class LLVMIRBuilder extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

export class LLVMModule extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

export class LLVMValue extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

export class LLVMType extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

export class LLVMBasicBlock extends UniqueType<Pointer<void>> {
  static TYPE = refType('void');

  private __name = this;
}

const LLVMTypeArray = arrayRef(LLVMType.TYPE);
const LLVMValueArray = arrayRef(LLVMValue.TYPE);

function buildArray<T extends UniqueType<Pointer<void>>>(
  type: ArrayType<T['value']>,
  values: T[],
): TypedArray<T['value']> {
  const ref = new type(values.length);
  for (const index in values) {
    ref[index] = values[index].value;
  }
  return ref;
}
