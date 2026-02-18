import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  init,
  taskKey,
  taskQueueAuthorityKey,
} from "@helium/tuktuk-sdk";
import { TuktukCounter } from "../target/types/tuktuk_counter";
import { assert } from "chai";

describe("tuktuk-counter", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.tuktukCounter as Program<TuktukCounter>;

  const taskQueue = new anchor.web3.PublicKey("CMreFdKxT5oeZhiX8nWTGz9PtXM1AMYTh6dGR2UzdtrA");
  const counter = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("counter")], program.programId)[0];
  const queueAuthority = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("queue_authority")], program.programId)[0];
  const taskQueueAuthority = taskQueueAuthorityKey(taskQueue, queueAuthority)[0];

  xit("Initialize counter", async () => {
    const tx = await program.methods.initialize()
    .accountsPartial({
      user: provider.publicKey,
      counter: counter,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();
    console.log("\nYour transaction signature", tx);
  });

  it("Increment counter", async () => {
    const tx = await program.methods.increment()
    .accountsPartial({
      counter: counter,
    })
    .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("\nQueue Authority PDA:", queueAuthority.toBase58());
    console.log("\nCounter Value:", (await program.account.counter.fetch(counter)).count.toString());
  });

  it("Schedule increment task", async () => {
    let tuktukProgram = await init(provider);

    let taskID = 6;
    const tx = await program.methods.schedule(taskID)
    .accountsPartial({
      user: provider.publicKey,
      counter: counter,
      taskQueue: taskQueue,
      taskQueueAuthority: taskQueueAuthority,
      task: taskKey(taskQueue, taskID)[0],
      queueAuthority: queueAuthority,
      systemProgram: anchor.web3.SystemProgram.programId,
      tuktukProgram: tuktukProgram.programId,
    })
    .rpc({skipPreflight: true});
    assert(tuktukProgram.programId.equals(new anchor.web3.PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA")));
    console.log("\nYour transaction signature", tx);
  });
});
