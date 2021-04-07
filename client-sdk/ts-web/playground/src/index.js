// @ts-check

import * as oasis from '@oasisprotocol/client';
import * as oasisRT from '@oasisprotocol/client-rt';

import * as oasisBridge from './../..';

const BRIDGE_RUNTIME_ID = oasis.misc.fromHex('8000000000000000000000000000000000000000000000000000000000000000');

const FEE_FREE = /** @type {oasisRT.types.BaseUnits} */ ([oasis.quantity.fromBigInt(0n), oasisRT.token.NATIVE_DENOMINATION]);

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

const nic = new oasis.client.NodeInternal('http://localhost:42280');
const accountsWrapper = new oasisRT.accounts.Wrapper(BRIDGE_RUNTIME_ID);
const bridgeWrapper = new oasisBridge.Wrapper(BRIDGE_RUNTIME_ID);

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} user
 * @param {oasisRT.types.BaseUnits} amount
 * @param {string} consensusChainContext
 */
async function userOut(label, user, amount, consensusChainContext) {
    console.log('out user', label, 'getting nonce');
    const nonce = await accountsWrapper.queryNonce()
        .setArgs({
            address: await oasis.staking.addressFromPublicKey(user.public()),
        })
        .query(nic);
    console.log('out user', label, 'nonce', nonce);
    const siUser = /** @type {oasisRT.types.SignerInfo} */ ({pub: {ed25519: user.public()}, nonce});

    const target = oasis.misc.fromHex('0000000000000000000000000000000000000000');

    console.log('out user', label, 'locking', amount);
    const tw = bridgeWrapper.callLock()
        .setBody({
            target,
            amount,
        })
        .setSignerInfo([siUser])
        .setFeeAmount(FEE_FREE)
        .setFeeGas(0n);
    await tw.sign([user], consensusChainContext);
    const lockResult = await tw.submit(nic);
    console.log('out user', label, 'lock result', lockResult);

    return lockResult.id
}

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} witness
 * @param {oasis.types.longnum} id
 * @param {oasisRT.types.BaseUnits} amount
 * @param {Uint8Array} target
 * @param {string} consensusChainContext
 */
async function witnessIn(label, witness, id, amount, target, consensusChainContext) {
    console.log('in witness', label, 'getting nonce');
    const nonce = await accountsWrapper.queryNonce()
        .setArgs({
            address: await oasis.staking.addressFromPublicKey(witness.public()),
        })
        .query(nic);
    console.log('in witness', label, 'nonce', nonce);
    const siWitness = /** @type {oasisRT.types.SignerInfo} */ ({pub: {ed25519: witness.public()}, nonce});

    console.log('in witness', label, 'releasing', id, amount, target);
    const tw = bridgeWrapper.callRelease()
        .setBody({
            id,
            amount,
            target,
        })
        .setSignerInfo([siWitness])
        .setFeeAmount(FEE_FREE)
        .setFeeGas(0n);
    await tw.sign([witness], consensusChainContext);
    await tw.submit(nic);
    console.log('in witness', label, 'release done');
}

/**
 * @param {string} label
 * @param {oasis.signature.ContextSigner} witness
 * @param {oasis.types.longnum} id
 * @param {string} consensusChainContext
 */
async function witnessOut(label, witness, id, consensusChainContext) {
    console.log('out witness', label, 'getting nonce');
    const nonce = await accountsWrapper.queryNonce()
        .setArgs({
            address: await oasis.staking.addressFromPublicKey(witness.public()),
        })
        .query(nic);
    console.log('out witness', label, 'nonce', nonce);
    const siWitness = /** @type {oasisRT.types.SignerInfo} */ ({pub: {ed25519: witness.public()}, nonce});

    const sig = oasis.misc.fromString(`signature:${btoa(String.fromCharCode.apply(null, witness.public()))}`);
    console.log('out witness', label, 'witnessing', id, sig);
    const tw = bridgeWrapper.callWitness()
        .setBody({
            id,
            sig,
        })
        .setSignerInfo([siWitness])
        .setFeeAmount(FEE_FREE)
        .setFeeGas(0n);
    await tw.sign([witness], consensusChainContext);
    await tw.submit(nic);
    console.log('out witness', label, 'witness done');
}

export const playground = (async function () {
    // Wait for ready.
    console.log('waiting for node to be ready');
    const waitStart = Date.now();
    await nic.nodeControllerWaitReady();
    const waitEnd = Date.now();
    console.log(`ready ${waitEnd - waitStart} ms`);

    // Get consensus chain context.
    const consensusChainContext = await nic.consensusGetChainContext();

    const alice = oasis.signature.NaclSigner.fromSeed(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: alice')), 'this key is not important');
    const bob = oasis.signature.NaclSigner.fromSeed(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: bob')), 'this key is not important');
    const charlie = oasis.signature.NaclSigner.fromSeed(await oasis.hash.hash(oasis.misc.fromString('oasis-runtime-sdk/test-keys: charlie')), 'this key is not important');

    const aliceAddress = await oasis.staking.addressFromPublicKey(alice.public());

    const lockWaiter = /** @type {BridgeWaiter<oasisBridge.LockEvent>} */ (new BridgeWaiter());
    const releaseWaiter = /** @type {BridgeWaiter<oasisBridge.ReleaseEvent>} */ (new BridgeWaiter());
    const witnessesSignedWaiter = /** @type {BridgeWaiter<oasisBridge.WitnessSignatures>} */ (new BridgeWaiter());

    // The user and witnesses are normally on different computers and would each watch blocks on their
    // own, but for simplicity in this example, we're running a single shared subscription.

    const eventVisitor = new oasisRT.event.Visitor([
        oasisRT.accounts.moduleEventHandler({
            [oasisRT.accounts.EVENT_TRANSFER_CODE]: (e, transferEvent) => {
                console.log('observed transfer', transferEvent);
            },
            [oasisRT.accounts.EVENT_BURN_CODE]: (e, releaseEvent) => {
                console.log('observed burn', releaseEvent);
            },
            [oasisRT.accounts.EVENT_MINT_CODE]: (e, mintEvent) => {
                console.log('observed mint', mintEvent);
            }
        }),
        oasisBridge.moduleEventHandler({
            [oasisBridge.EVENT_LOCK_CODE]: (e, lockEvent) => {
                console.log('observed lock', lockEvent);
                lockWaiter.observe(lockEvent.id, lockEvent);
            },
            [oasisBridge.EVENT_RELEASE_CODE]: (e, releaseEvent) => {
                console.log('observed release', releaseEvent);
                releaseWaiter.observe(releaseEvent.id, releaseEvent);
            },
            [oasisBridge.EVENT_WITNESSES_SIGNED_CODE]: (e, witnessSignatures) => {
                console.log('observed witnesses signed', witnessSignatures);
                witnessesSignedWaiter.observe(witnessSignatures.id, witnessSignatures);
            },
        }),
    ]);

    /**
     * @param {oasis.types.RoothashAnnotatedBlock} annotatedBlock
     */
    function handleBlock(annotatedBlock) {
        console.log('observed block', annotatedBlock.block.header.round);
        (async () => {
            try {
                /** @type oasis.types.RuntimeClientEvent[] */
                const events = await nic.runtimeClientGetEvents({
                    runtime_id: BRIDGE_RUNTIME_ID,
                    round: annotatedBlock.block.header.round,
                }) || [];
                for (const event of events) {
                    console.log('observed event', event);
                    eventVisitor.visit(event);
                }
            } catch (e) {
                console.error(e);
            }
        })();
    }

    const blocks = nic.runtimeClientWatchBlocks(BRIDGE_RUNTIME_ID);
    blocks.on('data', handleBlock);

    // Out flow.
    {
        console.log('out user locking');
        const id = await userOut('alice', new oasis.signature.BlindContextSigner(alice), [oasis.quantity.fromBigInt(10n), oasisRT.token.NATIVE_DENOMINATION], consensusChainContext);
        console.log('out waiting for lock event');
        await lockWaiter.wait(id);
        console.log('out witnesses signing');
        await witnessOut('bob', new oasis.signature.BlindContextSigner(bob), id, consensusChainContext);
        await witnessOut('charlie', new oasis.signature.BlindContextSigner(charlie), id, consensusChainContext);
        console.log('out waiting for witnesses signed event');
        await witnessesSignedWaiter.wait(id);
        console.log('out done');
    }
    // In flow.
    {
        console.log('in querying next sequence numbers');
        const numbers = await bridgeWrapper.queryNextSequenceNumbers()
            .query(nic);
        console.log('next sequence numbers', numbers);
        const localReleaseID = BigInt(numbers.in);
        const remoteReleaseID = BigInt(numbers.in) + 1n;

        // Local denomination.
        const localAmount = /** @type {oasisRT.types.BaseUnits} */ ([oasis.quantity.fromBigInt(10n), oasisRT.token.NATIVE_DENOMINATION]);
        console.log('in local witnesses signing');
        await witnessIn('bob', new oasis.signature.BlindContextSigner(bob), localReleaseID, localAmount, aliceAddress, consensusChainContext);
        await witnessIn('charlie', new oasis.signature.BlindContextSigner(charlie), localReleaseID, localAmount, aliceAddress, consensusChainContext);
        console.log('in local waiting for release event');
        await releaseWaiter.wait(localReleaseID);
        console.log('in local done');

        // Remote denomination.
        const remoteAmount = /** @type {oasisRT.types.BaseUnits} */ ([oasis.quantity.fromBigInt(10n), oasis.misc.fromString('oETH')]);
        console.log('in remote witnesses signing');
        await witnessIn('bob', new oasis.signature.BlindContextSigner(bob), remoteReleaseID, remoteAmount, aliceAddress, consensusChainContext);
        await witnessIn('charlie', new oasis.signature.BlindContextSigner(charlie), remoteReleaseID, remoteAmount, aliceAddress, consensusChainContext);
        console.log('in remote waiting for release event');
        await releaseWaiter.wait(remoteReleaseID);
        console.log('in remote done');
    }
    // Parameters.
    {
        console.log('bridge parameters', await bridgeWrapper.queryParameters().query(nic));
    }
})();

playground.catch((e) => {
    console.error(e);
});
