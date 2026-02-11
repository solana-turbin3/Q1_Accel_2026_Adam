import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { ErStateAccount } from "../target/types/er_state_account";

// VRF constants (from ephemeral_vrf_sdk::consts)
const VRF_PROGRAM_ID = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
const DEFAULT_QUEUE = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
const SLOT_HASHES_SYSVAR = new PublicKey("SysvarS1otHashes111111111111111111111111111");

describe("er-state-account", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  console.log("Connection: ", provider.connection.rpcEndpoint);
  console.log(`Wallet: ${anchor.Wallet.local().publicKey}`);

  const program = anchor.workspace.erStateAccount as Program<ErStateAccount>;

  const userAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user"), anchor.Wallet.local().publicKey.toBuffer()],
    program.programId,
  )[0];

  // Program identity PDA (used by the #[vrf] macro)
  const programIdentity = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("identity")],
    program.programId,
  )[0];

  before(async function () {
    const balance = await provider.connection.getBalance(
      anchor.Wallet.local().publicKey,
    );
    console.log("Balance:", balance / LAMPORTS_PER_SOL, "SOL");
    console.log("Program ID:", program.programId.toBase58());
    console.log("User Account PDA:", userAccount.toBase58());
    console.log("Program Identity PDA:", programIdentity.toBase58(), "\n");
  });

  // ── Basic flow ────────────────────────────────────────────────

  it("Initialize user account", async () => {
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("  tx:", tx);
  });

  it("Update state to 42", async () => {
    const tx = await program.methods
      .update(new anchor.BN(42))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();
    console.log("  tx:", tx);

    const account = await program.account.userAccount.fetch(userAccount);
    console.log("  data:", account.data.toString());
  });

  // ── VRF: Request Randomness (Task 1 — L1) ────────────────────

  it("Request randomness on L1 (enqueues VRF request)", async () => {
    const tx = await program.methods
      .requestRandomness(42) // client_seed = 42
      .accountsPartial({
        payer: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        oracleQueue: DEFAULT_QUEUE,
        programIdentity: programIdentity,
        vrfProgram: VRF_PROGRAM_ID,
        slotHashes: SLOT_HASHES_SYSVAR,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });
    console.log("  tx:", tx);
    console.log("  ✓ VRF request enqueued — waiting for oracle callback");
    console.log("    (no oracle running locally, so callback won't fire)");
  });

  // ── Cleanup ───────────────────────────────────────────────────

  it("Close account", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("  tx:", tx);
  });
});
