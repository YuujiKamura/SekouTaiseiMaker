declare module 'xlsx-populate' {
  interface Cell {
    value(): any;
    value(value: any): Cell;
  }

  interface Sheet {
    name(): string;
    cell(address: string): Cell;
  }

  interface OutputOptions {
    type?: 'nodebuffer' | 'blob' | 'arraybuffer' | 'base64' | 'uint8array';
    password?: string;
  }

  interface Workbook {
    sheet(nameOrIndex: string | number): Sheet | undefined;
    sheets(): Sheet[];
    outputAsync(options?: OutputOptions): Promise<ArrayBuffer | Blob | string | Uint8Array>;
  }

  function fromDataAsync(data: ArrayBuffer | Uint8Array): Promise<Workbook>;
  function fromFileAsync(path: string): Promise<Workbook>;

  export { Workbook, Sheet, Cell };
  export default { fromDataAsync, fromFileAsync };
}
