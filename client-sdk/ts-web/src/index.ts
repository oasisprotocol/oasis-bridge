import * as oasis from '@oasisprotocol/client';
import * as oasisRT from '@oasisprotocol/client-rt';

/**
 * Unique module name.
 */
export const MODULE_NAME = 'bridge';

export const ERR_INVALID_ARGUMENT_CODE = 1;
export const ERR_NOT_AUTHORIZED_CODE = 2;
export const ERR_INVALID_SEQUENCE_NUMBER_CODE = 3;
export const ERR_INSUFFICIENT_BALANCE_CODE = 4;
export const ERR_ALREADY_SUBMITTED_SIGNATURE_CODE = 5;
export const ERR_UNSUPPORTED_DENOMINATION_CODE = 6;

// Callable methods.
export const METHOD_LOCK = 'bridge.Lock';
export const METHOD_WITNESS = 'bridge.Witness';
export const METHOD_RELEASE = 'bridge.Release';
// Queries.
export const METHOD_NEXT_SEQUENCE_NUMBERS = 'bridge.NextSequenceNumbers';
export const METHOD_PARAMETERS = 'bridge.Parameters';

export const EVENT_LOCK_CODE = 1;
export const EVENT_RELEASE_CODE = 2;
export const EVENT_WITNESSES_SIGNED_CODE = 3;

/**
 * Lock call.
 */
export interface Lock {
    target: Uint8Array;
    amount: oasisRT.types.BaseUnits;
}

export interface LockEvent {
    id: oasis.types.longnum;
    owner: Uint8Array;
    target: Uint8Array;
    amount: oasisRT.types.BaseUnits;
}

/**
 * Lock call results.
 */
export interface LockResult {
    id: oasis.types.longnum;
}

/**
 * Next event sequence numbers.
 */
export interface NextSequenceNumbers {
    in: oasis.types.longnum;
    out: oasis.types.longnum;
}

/**
 * Operation.
 */
export interface Operation {
    lock?: Lock;
    release?: Release;
}

/**
 * Parameters for the bridge module.
 */
export interface Parameters {
    /**
     * A list of authorized witness public keys.
     */
    witnesses: oasisRT.types.PublicKey[];
    /**
     * Number of witnesses that needs to sign off.
     */
    threshold: oasis.types.longnum;
    /**
     * Denominations local to this side of the bridge.
     */
    local_denominations: Uint8Array[];
    /**
     * Denominations that exist on the remote side of the bridge.
     */
    remote_denominations: Map<Uint8Array, Uint8Array>;
}

/**
 * Release call.
 */
export interface Release {
    id: oasis.types.longnum;
    target: Uint8Array;
    amount: oasisRT.types.BaseUnits;
}

export interface ReleaseEvent {
    id: oasis.types.longnum;
    target: Uint8Array;
    amount: oasisRT.types.BaseUnits;
}

/**
 * Witness event call.
 */
export interface Witness {
    id: oasis.types.longnum;
    sig: Uint8Array;
}

/**
 * Outgoing witness signatures.
 */
export interface WitnessSignatures {
    id: oasis.types.longnum;
    op: Operation;
    wits?: number[];
    sigs?: Uint8Array[];
}

export class Wrapper extends oasisRT.wrapper.Base {

    constructor(runtimeID: Uint8Array) {
        super(runtimeID);
    }

    callLock() { return this.call<Lock, LockResult>(METHOD_LOCK); }
    callWitness() { return this.call<Witness, void>(METHOD_WITNESS); }
    callRelease() { return this.call<Release, void>(METHOD_RELEASE); }

    queryNextSequenceNumbers() { return this.query<void, NextSequenceNumbers>(METHOD_NEXT_SEQUENCE_NUMBERS); }
    queryParameters() { return this.query<void, Parameters>(METHOD_PARAMETERS); }

}
