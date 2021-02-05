// @ts-expect-error missing declaration
import TransportWebUSB from '@ledgerhq/hw-transport-webusb';
// @ts-expect-error missing declaration
import OasisApp from '@oasisprotocol/ledger';

import * as oasisBridge from '../../ts-web';

interface Response {
    return_code: number;
    error_message: string;
    [index: string]: any;
}

function u8FromBuf(buf: Buffer) {
    return new Uint8Array(buf.buffer);
}

function bufFromU8(u8: Uint8Array) {
    return Buffer.from(u8.buffer, u8.byteOffset, u8.byteLength);
}

function successOrThrow(response: Response, message: string) {
    if (response.return_code !== 0x9000) throw new Error(`${message}: ${response.return_code} ${response.error_message}`);
    return response;
}

export class LedgerContextSigner implements oasisBridge.signature.ContextSigner {

    app: OasisApp;
    path: number[];
    publicKey: Uint8Array;

    constructor(app: number, path: number[], publicKey: Uint8Array) {
        this.app = app;
        this.path = path;
        this.publicKey = publicKey;
    }

    public(): Uint8Array {
        return this.publicKey;
    }

    async sign(context: string, message: Uint8Array): Promise<Uint8Array> {
        const response = successOrThrow(await this.app.sign(this.path, context, bufFromU8(message)), 'ledger sign');
        return u8FromBuf(response.signature);
    }

    static async fromWebUSB(keyNumber: number) {
        const transport = await TransportWebUSB.create();
        const app = new OasisApp(transport);
        // Specification forthcoming. See https://github.com/oasisprotocol/oasis-core/pull/3656.
        const path = [44, 474, 0, 0, keyNumber];
        const publicKeyResponse = successOrThrow(await app.publicKey(path), 'ledger public key');
        return new LedgerContextSigner(app, path, u8FromBuf(publicKeyResponse.pk));
    }

}
