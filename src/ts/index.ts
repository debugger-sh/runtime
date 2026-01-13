// import runtime from '../../pkg/runtime_bg.wasm';

export type Lang = 'c';

export type ExecutionResult = never;

export class Runtime {
  static async create(lang: Lang): Promise<void> {
    // await (runtime as unknown as () => Promise<>)().then((rt) => rt.main());
    throw new Error(lang);
  }

  async run(): Promise<ExecutionResult> {
    throw new Error();
  }

  /* visit later: 
        runtime.stdout.pipeTo(console.log);
        runtime.stdin.write("haha ");
   */
}
