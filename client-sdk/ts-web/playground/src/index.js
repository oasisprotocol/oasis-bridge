// @ts-check

import * as oasis from '@oasisprotocol/client';
import * as oasisRT from '@oasisprotocol/client-rt';

import * as oasisBridge from './../..';

const BRIDGE_RUNTIME_ID = oasis.misc.fromHex('8000000000000000000000000000000000000000000000000000000000000000');
const LOCK_EVENT_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisBridge.MODULE_NAME, oasisBridge.EVENT_LOCK_CODE));
const RELEASE_EVENT_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisBridge.MODULE_NAME, oasisBridge.EVENT_RELEASE_CODE));
const WITNESSES_SIGNED_EVENT_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisBridge.MODULE_NAME, oasisBridge.EVENT_WITNESSES_SIGNED_CODE));
const TRANSFER_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisRT.accounts.MODULE_NAME, oasisRT.accounts.EVENT_TRANSFER_CODE));
const BURN_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisRT.accounts.MODULE_NAME, oasisRT.accounts.EVENT_BURN_CODE));
const MINT_TAG_HEX = oasis.misc.toHex(oasisRT.event.toTag(oasisRT.accounts.MODULE_NAME, oasisRT.accounts.EVENT_MINT_CODE));

/**
 * @template E
 */
class BridgeWaiter {

    constructor() {
        this.promised = /** @type {{[idStr: string]: Promise<E>}} */ ({});
        this.requested = /** @type {{[idStr: string]: {resolve: (event: E) => void, reject: (reason: any) => void}}} */ ({});
    }

    /**
     * @param {oasis.types.longnum} id
     */
    wait(id) {
        const idStr = '' + id;
        if (!(idStr in this.promised)) {
            this.promised[idStr] = new Promise((resolve, reject) => {
                this.requested[idStr] = {resolve, reject};
            });
        }
        return this.promised[idStr];
    }

    /**
     * @param {oasis.types.longnum} id
     * @param {E} event
     */
    observe(id, event) {
        const idStr = '' + id;
        if (idStr in this.requested) {
            const {resolve, reject} = this.requested[idStr];
            delete this.requested[idStr];
            resolve(event);
        } else {
            this.promised[idStr] = Promise.resolve(event);
        }
    }

}

const client = new oasis.OasisNodeClient('http://localhost:42280');

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} user
 * @param {oasisRT.types.BaseUnits} amount
 */
async function userOut(label, user, amount) {
    console.log('out user', label, 'getting nonce');
    const nonce = /** @type {oasis.types.longnum} */ ((await client.runtimeClientQuery({
        runtime_id: BRIDGE_RUNTIME_ID,
        round: oasis.runtime.CLIENT_ROUND_LATEST,
        method: oasisRT.accounts.METHOD_NONCE,
        args: /** @type {oasisRT.types.AccountsNonceQuery} */ ({
            address: await oasis.staking.addressFromPublicKey(user.public()),
        }),
    })).data);
    console.log('out user', label, 'nonce', nonce);

    const transaction = /** @type {oasisRT.types.Transaction} */ ({
        v: oasisRT.transaction.LATEST_TRANSACTION_VERSION,
        call: {
            method: oasisBridge.METHOD_LOCK,
            body: /** @type {oasisBridge.Lock} */ ({
                amount,
            }),
        },
        ai: {
            si: [{pub: {ed25519: user.public()}, nonce}],
            fee: {
                amount: [oasis.quantity.fromBigInt(0n), oasisRT.token.NATIVE_DENOMINATION],
                gas: 0,
            },
        },
    });
    console.log('out user', label, 'transaction', transaction);
    const signed = await oasisRT.transaction.signUnverifiedTransaction([user], transaction);
    console.log('out user', label, 'signed', signed);
    const result = /** @type {oasisRT.types.CallResult} */ (oasis.misc.fromCBOR(await client.runtimeClientSubmitTx({
        runtime_id: BRIDGE_RUNTIME_ID,
        data: oasis.misc.toCBOR(signed),
    })));
    console.log('out user', label, 'result', result);
    if (result.fail) throw result.fail;
    const lockResult = /** @type {oasisBridge.LockResult} */ (result.ok);

    return lockResult.id
}

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} witness
 * @param {oasis.types.longnum} id
 * @param {oasisRT.types.BaseUnits} amount
 * @param {Uint8Array} owner
 */
async function witnessIn(label, witness, id, amount, owner) {
    console.log('in witness', label, 'getting nonce');
    const nonce = /** @type {oasis.types.longnum} */ ((await client.runtimeClientQuery({
        runtime_id: BRIDGE_RUNTIME_ID,
        round: oasis.runtime.CLIENT_ROUND_LATEST,
        method: oasisRT.accounts.METHOD_NONCE,
        args: /** @type {oasisRT.types.AccountsNonceQuery} */ ({
            address: await oasis.staking.addressFromPublicKey(witness.public()),
        }),
    })).data);
    console.log('in witness', label, 'nonce', nonce);

    const transaction = /** @type {oasisRT.types.Transaction} */ ({
        v: oasisRT.transaction.LATEST_TRANSACTION_VERSION,
        call: {
            method: oasisBridge.METHOD_RELEASE,
            body: /** @type {oasisBridge.Release} */ ({
                id,
                amount,
                owner,
            }),
        },
        ai: {
            si: [{pub: {ed25519: witness.public()}, nonce}],
            fee: {
                amount: [oasis.quantity.fromBigInt(0n), oasisRT.token.NATIVE_DENOMINATION],
                gas: 0,
            },
        },
    });
    console.log('in witness', label, 'transaction', transaction);
    const signed = await oasisRT.transaction.signUnverifiedTransaction([witness], transaction);
    console.log('in witness', label, 'signed', signed);
    const result = /** @type {oasisRT.types.CallResult} */ (oasis.misc.fromCBOR(await client.runtimeClientSubmitTx({
        runtime_id: BRIDGE_RUNTIME_ID,
        data: oasis.misc.toCBOR(signed),
    })));
    console.log('in witness', label, 'result', result);
    if (result.fail) throw result.fail;
}

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} witness
 * @param {oasis.types.longnum} id
 */
async function witnessOut(label, witness, id) {
    console.log('out witness', label, 'getting nonce');
    const nonce = /** @type {oasis.types.longnum} */ ((await client.runtimeClientQuery({
        runtime_id: BRIDGE_RUNTIME_ID,
        round: oasis.runtime.CLIENT_ROUND_LATEST,
        method: oasisRT.accounts.METHOD_NONCE,
        args: /** @type {oasisRT.types.AccountsNonceQuery} */ ({
            address: await oasis.staking.addressFromPublicKey(witness.public()),
        }),
    })).data);
    console.log('out witness', label, 'nonce', nonce);

    const transaction = /** @type {oasisRT.types.Transaction} */ ({
        v: oasisRT.transaction.LATEST_TRANSACTION_VERSION,
        call: {
            method: oasisBridge.METHOD_WITNESS,
            body: /** @type {oasisBridge.Witness} */ ({
                id,
                // todo: Reconcile this with module-bridge/src/lib.rs.
                sig: oasis.misc.fromString(`signature:${btoa(String.fromCharCode.apply(null, witness.public()))}`),
            }),
        },
        ai: {
            si: [{pub: {ed25519: witness.public()}, nonce}],
            fee: {
                amount: [oasis.quantity.fromBigInt(0n), oasisRT.token.NATIVE_DENOMINATION],
                gas: 0,
            },
        },
    });
    console.log('out witness', label, 'transaction', transaction);
    const signed = await oasisRT.transaction.signUnverifiedTransaction([witness], transaction);
    console.log('out witness', label, 'signed', signed);
    const result = /** @type {oasisRT.types.CallResult} */ (oasis.misc.fromCBOR(await client.runtimeClientSubmitTx({
        runtime_id: BRIDGE_RUNTIME_ID,
        data: oasis.misc.toCBOR(signed),
    })));
    console.log('out witness', label, 'result', result);
    if (result.fail) throw result.fail;
}

(async function () {
    try {
        const alice = oasis.signature.EllipticSigner.fromSecret(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: alice')), 'this key is not important');
        const bob = oasis.signature.EllipticSigner.fromSecret(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: bob')), 'this key is not important');
        const charlie = oasis.signature.EllipticSigner.fromSecret(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: charlie')), 'this key is not important');

        const aliceAddress = await oasis.staking.addressFromPublicKey(alice.public());

        const lockWaiter = /** @type {BridgeWaiter<oasisBridge.LockEvent>} */ (new BridgeWaiter());
        const releaseWaiter = /** @type {BridgeWaiter<oasisBridge.ReleaseEvent>} */ (new BridgeWaiter());
        const witnessesSignedWaiter = /** @type {BridgeWaiter<oasisBridge.WitnessSignatures>} */ (new BridgeWaiter());

        // The user and witnesses are normally on different computers and would each watch blocks on their
        // own, but for simplicity in this example, we're running a single shared subscription.

        /**
         * @param {oasisBridge.LockEvent} lockEvent
         */
        function handleLockEvent(lockEvent) {
            console.log('observed lock', lockEvent);
            lockWaiter.observe(lockEvent.id, lockEvent);
        }

        /**
         * @param {oasisBridge.ReleaseEvent} releaseEvent
         */
        function handleReleaseEvent(releaseEvent) {
            console.log('observed release', releaseEvent);
            releaseWaiter.observe(releaseEvent.id, releaseEvent);
        }

        /**
         * @param {oasisBridge.WitnessSignatures} witnessSignatures
         */
        function handleWitnessesSignedEvent(witnessSignatures) {
            console.log('observed witnesses signed', witnessSignatures);
            witnessesSignedWaiter.observe(witnessSignatures.id, witnessSignatures);
        }

        /**
         * @param {oasisRT.types.AccountsTransferEvent} transferEvent
         */
        function handleTransferEvent(transferEvent) {
            console.log('observed transfer', transferEvent);
        }

        /**
         * @param {oasisRT.types.AccountsBurnEvent} burnEvent
         */
        function handleBurnEvent(burnEvent) {
            console.log('observed burn', burnEvent);
        }

        /**
         * @param {oasisRT.types.AccountsMintEvent} mintEvent
         */
        function handleMintEvent(mintEvent) {
            console.log('observed mint', mintEvent);
        }

        /**
         * @param {oasis.types.RuntimeClientEvent} event
         */
        function handleEvent(event) {
            console.log('observed event', event);
            switch (oasis.misc.toHex(event.key)) {
                case LOCK_EVENT_TAG_HEX:
                    handleLockEvent(/** @type {oasisBridge.LockEvent} */ (oasis.misc.fromCBOR(event.value)));
                    break;
                case RELEASE_EVENT_TAG_HEX:
                    handleReleaseEvent(/** @type {oasisBridge.ReleaseEvent} */ (oasis.misc.fromCBOR(event.value)));
                    break;
                case WITNESSES_SIGNED_EVENT_TAG_HEX:
                    handleWitnessesSignedEvent(/** @type {oasisBridge.WitnessSignatures} */ (oasis.misc.fromCBOR(event.value)));
                    break;
                case TRANSFER_TAG_HEX:
                    handleTransferEvent(/** @type {oasisRT.types.AccountsTransferEvent} */ (oasis.misc.fromCBOR(event.value)));
                    break;
                case BURN_TAG_HEX:
                    handleBurnEvent(/** @type {oasisRT.types.AccountsBurnEvent} */ (oasis.misc.fromCBOR(event.value)));
                    break;
                case MINT_TAG_HEX:
                    handleMintEvent(/** @type {oasisRT.types.AccountsMintEvent} */ (oasis.misc.fromCBOR(event.value)));
                    break;
            }
        }

        /**
         * @param {oasis.types.RoothashAnnotatedBlock} annotatedBlock
         */
        function handleBlock(annotatedBlock) {
            console.log('observed block', annotatedBlock.block.header.round);
            (async () => {
                try {
                    /** @type oasis.types.RuntimeClientEvent[] */
                    let events;
                    try {
                        events = await client.runtimeClientGetEvents({
                            runtime_id: BRIDGE_RUNTIME_ID,
                            round: annotatedBlock.block.header.round,
                        });
                    } catch (e) {
                        if (e.message === 'Incomplete response') {
                            // This is normal. grpc-web freaks out if the response is `== null`, which it
                            // always is for a void method.
                            // todo: unhack this when they release with our change
                            // https://github.com/grpc/grpc-web/pull/1025
                            events = [];
                        } else {
                            throw e;
                        }
                    }
                    for (const event of events) {
                        handleEvent(event);
                    }
                } catch (e) {
                    console.error(e);
                }
            })();
        }

        const blocks = client.runtimeClientWatchBlocks(BRIDGE_RUNTIME_ID);
        blocks.on('data', handleBlock);

        // Out flow.
        {
            console.log('out user locking');
            const id = await userOut('alice', new oasis.signature.BlindContextSigner(alice), [oasis.quantity.fromBigInt(10n), oasisRT.token.NATIVE_DENOMINATION]);
            console.log('out waiting for lock event');
            await lockWaiter.wait(id);
            console.log('out witnesses signing');
            await witnessOut('bob', new oasis.signature.BlindContextSigner(bob), id);
            await witnessOut('charlie', new oasis.signature.BlindContextSigner(charlie), id);
            console.log('out waiting for witnesses signed event');
            await witnessesSignedWaiter.wait(id);
            console.log('out done');
        }
        // In flow.
        {
            console.log('in querying next sequence numbers');
            const numbers = /** @type {oasisBridge.NextSequenceNumbers} */ ((await client.runtimeClientQuery({
                runtime_id: BRIDGE_RUNTIME_ID,
                round: oasis.runtime.CLIENT_ROUND_LATEST,
                method: oasisBridge.METHOD_NEXT_SEQUENCE_NUMBERS,
                args: undefined,
            })).data);
            console.log('next sequence numbers', numbers);
            const localReleaseID = BigInt(numbers.in);
            const remoteReleaseID = BigInt(numbers.in) + 1n;

            // Local denomination.
            const localAmount = /** @type {oasisRT.types.BaseUnits} */ ([oasis.quantity.fromBigInt(10n), oasisRT.token.NATIVE_DENOMINATION]);
            console.log('in local witnesses signing');
            await witnessIn('bob', new oasis.signature.BlindContextSigner(bob), localReleaseID, localAmount, aliceAddress);
            await witnessIn('charlie', new oasis.signature.BlindContextSigner(charlie), localReleaseID, localAmount, aliceAddress);
            console.log('in local waiting for release event');
            await releaseWaiter.wait(localReleaseID);
            console.log('in local done');

            // Remote denomination.
            const remoteAmount = /** @type {oasisRT.types.BaseUnits} */ ([oasis.quantity.fromBigInt(10n), oasis.misc.fromString('oETH')]);
            console.log('in remote witnesses signing');
            await witnessIn('bob', new oasis.signature.BlindContextSigner(bob), remoteReleaseID, remoteAmount, aliceAddress);
            await witnessIn('charlie', new oasis.signature.BlindContextSigner(charlie), remoteReleaseID, remoteAmount, aliceAddress);
            console.log('in remote waiting for release event');
            await releaseWaiter.wait(remoteReleaseID);
            console.log('in remote done');
        }
    } catch (e) {
        console.error(e);
    }
})();
