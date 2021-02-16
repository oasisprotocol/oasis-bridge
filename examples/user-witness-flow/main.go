package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"os"
	"sync"

	"google.golang.org/grpc"

	"github.com/oasisprotocol/oasis-core/go/common"
	"github.com/oasisprotocol/oasis-core/go/common/cbor"
	cmnGrpc "github.com/oasisprotocol/oasis-core/go/common/grpc"
	"github.com/oasisprotocol/oasis-core/go/common/logging"
	"github.com/oasisprotocol/oasis-core/go/common/quantity"

	sdk "github.com/oasisprotocol/oasis-sdk/client-sdk/go"
	"github.com/oasisprotocol/oasis-sdk/client-sdk/go/client"
	"github.com/oasisprotocol/oasis-sdk/client-sdk/go/crypto/signature"
	"github.com/oasisprotocol/oasis-sdk/client-sdk/go/modules/accounts"
	"github.com/oasisprotocol/oasis-sdk/client-sdk/go/testing"
	"github.com/oasisprotocol/oasis-sdk/client-sdk/go/types"
)

var logger = logging.GetLogger("user-witness-flow")

// GrpcAddrEnvVar is the name of the environment variable that specifies the
// gRPC host address of the Oasis node that the client should connect to.
const GrpcAddrEnvVar = "OASIS_NODE_GRPC_ADDR"

// RuntimeIDEnvVar is the name of the environment variable that specifies the
// runtime identifier of the bridge runtime.
const RuntimeIDEnvVar = "BRIDGE_RUNTIME_ID"

// Return the value of the given environment variable or exit if it is
// empty (or unset).
func getEnvVarOrExit(name string) string {
	value := os.Getenv(name)
	if value == "" {
		logger.Error("environment variable missing",
			"name", name,
		)
		os.Exit(1)
	}
	return value
}

// TODO: Move these somewhere bridge-module specific.

// Lock is the body of the Lock call.
type Lock struct {
	Amount types.BaseUnits `json:"amount"`
}

// LockResult is the result of a Lock method call.
type LockResult struct {
	ID uint64 `json:"id"`
}

// Witness is the body of a Witness call.
type Witness struct {
	ID        uint64 `json:"id"`
	Signature []byte `json:"sig"`
}

// Release is the body of a Release call.
type Release struct {
	ID     uint64          `json:"id"`
	Owner  types.Address   `json:"owner"`
	Amount types.BaseUnits `json:"amount"`
}

// LockEvent is a lock event.
type LockEvent struct {
	ID     uint64          `json:"id"`
	Owner  types.Address   `json:"owner"`
	Amount types.BaseUnits `json:"amount"`
}

// LockEventKey is the key used for lock events.
var LockEventKey = sdk.NewEventKey("bridge", 1)

// ReleaseEvent is the release event.
type ReleaseEvent struct {
	ID     uint64          `json:"id"`
	Owner  types.Address   `json:"owner"`
	Amount types.BaseUnits `json:"amount"`
}

// ReleaseEventKey is the key used for release events.
var ReleaseEventKey = sdk.NewEventKey("bridge", 2)

// Operation is a bridge operation.
type Operation struct {
	Lock    *Lock    `json:"lock,omitempty"`
	Release *Release `json:"release,omitempty"`
}

// WitnessesSignedEvent is the witnesses signed event.
type WitnessesSignedEvent struct {
	ID         uint64    `json:"id"`
	Op         Operation `json:"op"`
	Witnesses  []uint16  `json:"wits,omitempty"`
	Signatures [][]byte  `json:"sigs,omitempty"`
}

// WitnessesSignedEventKey is the key used for witnesses signed events.
var WitnessesSignedEventKey = sdk.NewEventKey("bridge", 3)

// NextSequenceNumbers are the next sequence numbers.
type NextSequenceNumbers struct {
	Incoming uint64 `json:"in"`
	Outgoing uint64 `json:"out"`
}

// Client is a bridge runtime client.
type Client struct {
	client.RuntimeClient

	Accounts accounts.V1
}

// user is an example user flow.
func user(
	ctx context.Context,
	wg *sync.WaitGroup,
	rc *Client,
	signer signature.Signer,
) {
	logger := logger.With("side", "user")

	defer func() {
		logger.Info("done")
		wg.Done()
	}()

	// Subscribe to blocks.
	blkCh, blkSub, err := rc.WatchBlocks(ctx)
	if err != nil {
		logger.Error("failed to subscribe to runtime blocks",
			"err", err,
		)
		return
	}
	defer blkSub.Close()

	// Get nonce.
	nonce, err := rc.Accounts.Nonce(ctx, client.RoundLatest, types.NewAddress(signer.Public()))
	if err != nil {
		logger.Error("failed to fetch account nonce",
			"err", err,
		)
		return
	}

	// Submit Lock.
	logger.Info("submitting lock transaction")
	tx := types.NewTransaction(nil, "bridge.Lock", Lock{
		Amount: types.NewBaseUnits(*quantity.NewFromUint64(10), types.NativeDenomination),
	})
	tx.AppendSignerInfo(signer.Public(), nonce)
	tb := tx.PrepareForSigning()
	if err = tb.AppendSign(signer); err != nil {
		logger.Error("failed to sign lock transaction",
			"err", err,
		)
		return
	}
	raw, err := rc.SubmitTx(ctx, tb.UnverifiedTransaction())
	if err != nil {
		logger.Error("failed to submit lock transaction",
			"err", err,
		)
		return
	}

	// Deserialize call result and extract id.
	var lockResult LockResult
	if err = cbor.Unmarshal(raw, &lockResult); err != nil {
		logger.Error("failed to unmarshal lock result",
			"err", err,
		)
		return
	}
	lockID := lockResult.ID

	// Wait for a WitnessesSigned event.
	for {
		select {
		case <-ctx.Done():
			return
		case blk, ok := <-blkCh:
			if !ok {
				return
			}
			logger.Debug("seen new block",
				"round", blk.Block.Header.Round,
			)

			events, err := rc.GetEvents(ctx, blk.Block.Header.Round)
			if err != nil {
				logger.Error("failed to get events",
					"err", err,
					"round", blk.Block.Header.Round,
				)
				return
			}

			for _, ev := range events {
				// TODO: Have wrappers for converting events.
				logger.Debug("got event",
					"key", base64.StdEncoding.EncodeToString(ev.Key),
					"value", base64.StdEncoding.EncodeToString(ev.Value),
				)

				switch {
				case WitnessesSignedEventKey.IsEqual(ev.Key):
					var witnessEv WitnessesSignedEvent
					if err = cbor.Unmarshal(ev.Value, &witnessEv); err != nil {
						logger.Error("failed to unmarshal witnesses signed event",
							"err", err,
						)
						continue
					}

					logger.Debug("got witnesses signed event",
						"id", witnessEv.ID,
					)

					if witnessEv.ID == lockID {
						// Our lock has been witnessed.
						// TODO: Take the signatures and submit to the other side.
						logger.Info("got witness signatures",
							"sigs", witnessEv.Signatures,
						)
						return
					}
				default:
				}
			}
		}
	}
}

func showBalances(ctx context.Context, rc *Client, address types.Address) {
	rsp, err := rc.Accounts.Balances(ctx, client.RoundLatest, address)
	if err != nil {
		logger.Error("failed to fetch account balances",
			"err", err,
		)
		return
	}

	fmt.Printf("=== Balances for %s ===\n", address)
	for denom, balance := range rsp.Balances {
		fmt.Printf("%s: %s\n", denom, balance)
	}
	fmt.Printf("\n")
}

// witness is an example witness flow.
func witness(
	ctx context.Context,
	wg *sync.WaitGroup,
	releaseWg *sync.WaitGroup,
	rc *Client,
	signer signature.Signer,
) {
	logger := logger.With("side", "witness")

	defer func() {
		logger.Info("done")
		wg.Done()
	}()

	// Subscribe to blocks.
	blkCh, blkSub, err := rc.WatchBlocks(ctx)
	if err != nil {
		logger.Error("failed to subscribe to runtime blocks",
			"err", err,
		)
		return
	}
	defer blkSub.Close()

	var lastUser types.Address

	// TODO: Logic for persisting at which block we left off and back-processing any missed events.
WitnessAnEvent:
	for {
		select {
		case <-ctx.Done():
			return
		case blk, ok := <-blkCh:
			if !ok {
				return
			}
			logger.Debug("seen new block",
				"round", blk.Block.Header.Round,
			)

			events, err := rc.GetEvents(ctx, blk.Block.Header.Round)
			if err != nil {
				logger.Error("failed to get events",
					"err", err,
					"round", blk.Block.Header.Round,
				)
				return
			}

			// Collect lock events.
			var lockEvents []*LockEvent
			for _, ev := range events {
				// TODO: Have wrappers for converting events.
				logger.Debug("got event",
					"key", base64.StdEncoding.EncodeToString(ev.Key),
					"value", base64.StdEncoding.EncodeToString(ev.Value),
				)

				switch {
				case LockEventKey.IsEqual(ev.Key):
					var lockEv LockEvent
					if err = cbor.Unmarshal(ev.Value, &lockEv); err != nil {
						logger.Error("failed to unmarshal lock event",
							"err", err,
						)
						continue
					}

					logger.Debug("got lock event",
						"id", lockEv.ID,
						"owner", lockEv.Owner,
						"amount", lockEv.Amount,
					)

					lockEvents = append(lockEvents, &lockEv)
				default:
				}
			}

			if len(lockEvents) == 0 {
				continue
			}

			// Submit bridge.Witness transactions.
			for _, ev := range lockEvents {
				// TODO: Sign the event using witness key.
				evSignature := []byte("signature:" + signer.Public().String())

				logger.Info("submitting witness transaction",
					"id", ev.ID,
				)

				// Get nonce.
				nonce, err := rc.Accounts.Nonce(ctx, client.RoundLatest, types.NewAddress(signer.Public()))
				if err != nil {
					logger.Error("failed to fetch account nonce",
						"err", err,
					)
					return
				}

				tx := types.NewTransaction(nil, "bridge.Witness", Witness{
					ID:        ev.ID,
					Signature: evSignature,
				})
				tx.AppendSignerInfo(signer.Public(), nonce)
				tb := tx.PrepareForSigning()
				if err = tb.AppendSign(signer); err != nil {
					logger.Error("failed to sign witness transaction",
						"err", err,
					)
					return
				}
				_, err = rc.SubmitTx(ctx, tb.UnverifiedTransaction())
				if err != nil {
					logger.Error("failed to submit witness transaction",
						"err", err,
					)
					return
				}

				lastUser = ev.Owner
			}

			logger.Info("successfully witnessed events")

			// We only witness a single event.
			break WitnessAnEvent
		}
	}

	// Simulate release.
	logger.Info("simulating release")

	// Get nonce.
	nonce, err := rc.Accounts.Nonce(ctx, client.RoundLatest, types.NewAddress(signer.Public()))
	if err != nil {
		logger.Error("failed to fetch account nonce",
			"err", err,
		)
		return
	}

	// Get remote sequence number.
	// TODO: Move these to bridge-specific helpers.
	var sequences NextSequenceNumbers
	err = rc.Query(ctx, client.RoundLatest, "bridge.NextSequenceNumbers", nil, &sequences)
	if err != nil {
		logger.Error("failed to query next sequence numbers",
			"err", err,
		)
		return
	}

	tx := types.NewTransaction(nil, "bridge.Release", Release{
		ID:     sequences.Incoming,
		Owner:  lastUser,
		Amount: types.NewBaseUnits(*quantity.NewFromUint64(10), types.NativeDenomination),
	})
	tx.AppendSignerInfo(signer.Public(), nonce)
	tb := tx.PrepareForSigning()
	if err = tb.AppendSign(signer); err != nil {
		logger.Error("failed to sign release transaction",
			"err", err,
		)
		return
	}
	_, err = rc.SubmitTx(ctx, tb.UnverifiedTransaction())
	if err != nil {
		logger.Error("failed to submit release transaction",
			"err", err,
		)
		return
	}

	logger.Info("release successful")

	// Make sure all witnesses release before proceeding to make sure the bridge is ready for the
	// next release (e.g., the sequence number is incremented).
	releaseWg.Done()
	releaseWg.Wait()

	// Simulating remote release.
	logger.Info("simulating remote release")

	// Get nonce.
	nonce, err = rc.Accounts.Nonce(ctx, client.RoundLatest, types.NewAddress(signer.Public()))
	if err != nil {
		logger.Error("failed to fetch account nonce",
			"err", err,
		)
		return
	}

	tx = types.NewTransaction(nil, "bridge.Release", Release{
		ID:     sequences.Incoming + 1,
		Owner:  lastUser,
		Amount: types.NewBaseUnits(*quantity.NewFromUint64(10), types.Denomination("oETH")),
	})
	tx.AppendSignerInfo(signer.Public(), nonce)
	tb = tx.PrepareForSigning()
	if err = tb.AppendSign(signer); err != nil {
		logger.Error("failed to sign release transaction",
			"err", err,
		)
		return
	}
	_, err = rc.SubmitTx(ctx, tb.UnverifiedTransaction())
	if err != nil {
		logger.Error("failed to submit release transaction",
			"err", err,
		)
		return
	}

	logger.Info("remote release successful")
}

func main() {
	// Initialize logging.
	if err := logging.Initialize(os.Stdout, logging.FmtLogfmt, logging.LevelDebug, nil); err != nil {
		fmt.Fprintf(os.Stderr, "ERROR: Unable to initialize logging: %v\n", err)
		os.Exit(1)
	}

	// Load node address.
	addr := getEnvVarOrExit(GrpcAddrEnvVar)
	// Load bridge runtime ID.
	var runtimeID common.Namespace
	if err := runtimeID.UnmarshalHex(getEnvVarOrExit(RuntimeIDEnvVar)); err != nil {
		logger.Error("malformed runtime ID",
			"err", err,
		)
		os.Exit(1)
	}

	// TODO: Provide client SDK wrapper for establishing connections.
	// Establish new gRPC connection with the node.
	logger.Debug("establishing connection", "addr", addr)
	conn, err := cmnGrpc.Dial(addr, grpc.WithInsecure())
	if err != nil {
		logger.Error("Failed to establish connection",
			"addr", addr,
			"err", err,
		)
		os.Exit(1)
	}
	defer conn.Close()

	// Create the runtime client.
	c := client.New(conn, runtimeID)
	rc := &Client{
		RuntimeClient: c,
		Accounts:      accounts.NewV1(c),
	}

	// Start witness and user.
	var wg, releaseWg sync.WaitGroup
	wg.Add(3)        // 2 witnesses, 1 user
	releaseWg.Add(2) // 2 witnesses
	ctx := context.Background()

	// Start two witnesses.
	go witness(ctx, &wg, &releaseWg, rc, testing.Bob.Signer)
	go witness(ctx, &wg, &releaseWg, rc, testing.Charlie.Signer)
	// Start one user.
	go user(ctx, &wg, rc, testing.Alice.Signer)

	wg.Wait()

	// Show closing balances.
	showBalances(ctx, rc, testing.Alice.Address)

	logger.Info("all done")
}
