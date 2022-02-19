import * as web3 from '@solana/web3.js';
import * as anchor from '@project-serum/anchor';
import * as token from '@solana/spl-token';

import { Program } from '@project-serum/anchor';
import { AnchorStake } from '../target/types/anchor_stake';

describe('anchor_stake', async () => {

  // Configure the client to use the local cluster.
  let provider = anchor.Provider.env();
  let connection = provider.connection;
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorStake as Program<AnchorStake>;
  
  // create a new token mint X 
  let wallet = web3.Keypair.generate(); 
  let tx = await connection.requestAirdrop(
      wallet.publicKey,
      web3.LAMPORTS_PER_SOL * 100,
  );
  await connection.confirmTransaction(tx);

  // create a new mint to send to program 
  let mint_kp = web3.Keypair.generate();
  await token.createMint(
    connection, 
    wallet, 
    mint_kp.publicKey, 
    null, 
    18, 
    mint_kp, undefined, token.TOKEN_PROGRAM_ID
  )
  // init wallet with tokens 
  let wallet_x = await token.createAssociatedTokenAccount(connection, wallet, mint_kp.publicKey, provider.wallet.publicKey, null, token.TOKEN_PROGRAM_ID, token.ASSOCIATED_TOKEN_PROGRAM_ID)
  let init_x = 100
  await token.mintTo(connection, wallet, mint_kp.publicKey, wallet_x, mint_kp, init_x, [], null, token.TOKEN_PROGRAM_ID)
  
  const [synth_x_pda, sb] = 
    await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("synthetic"), mint_kp.publicKey.toBuffer()],
      program.programId
    );

  const [vault_x_pda, vb] = 
    await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("vault"), mint_kp.publicKey.toBuffer()],
      program.programId
    );

    //b"reciept", token_x.key().as_ref(), sender.key().as_ref()
  const [reciept_pda, rb] = 
    await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("reciept"), 
        mint_kp.publicKey.toBuffer(),
        provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );

  // initialize vault 
  console.log('initializing...')
  let balance = await connection.getTokenAccountBalance(wallet_x)
  console.log('BOB token amount', balance.value.amount)
  console.log('')
  await program.rpc.initialize({
    accounts: {
      tokenX: mint_kp.publicKey, 
      syntheticX: synth_x_pda, 
      vaultX: vault_x_pda, 
      payer: provider.wallet.publicKey, 
      systemProgram: web3.SystemProgram.programId,
      tokenProgram: token.TOKEN_PROGRAM_ID, 
      associatedTokenProgram: token.ASSOCIATED_TOKEN_PROGRAM_ID, 
      rent: web3.SYSVAR_RENT_PUBKEY
    }, 
  })

  // create a new staker account for tokenX 
  await program.rpc.newStaker({ accounts: {
    tokenX: mint_kp.publicKey, 
    reciept: reciept_pda, 
    sender: provider.wallet.publicKey, 
    systemProgram: web3.SystemProgram.programId,
  }});

  // new synthetic account 
  let wallet_synth_x = await token.createAssociatedTokenAccount(connection, wallet, synth_x_pda, provider.wallet.publicKey, null, token.TOKEN_PROGRAM_ID, token.ASSOCIATED_TOKEN_PROGRAM_ID)

  // helper fcn 
  async function print_state() {
    let balance = await connection.getTokenAccountBalance(wallet_x)
    console.log('BOB token X amount', balance.value.amount)

    balance = await connection.getTokenAccountBalance(wallet_synth_x)
    console.log('BOB token synthX amount', balance.value.amount)

    balance = await connection.getTokenAccountBalance(vault_x_pda)
    console.log('VAULT token X amount', balance.value.amount)
  }
  
  let operation_accounts = { 
    tokenX: mint_kp.publicKey,  
    syntheticX: synth_x_pda, 
    vaultX: vault_x_pda, 
    sender: provider.wallet.publicKey, 
    senderTokenX: wallet_x,
    senderTokenSynthX: wallet_synth_x, 
    tokenProgram: token.TOKEN_PROGRAM_ID,
    clock: web3.SYSVAR_CLOCK_PUBKEY,
    reciept: reciept_pda,
  }

  // transfer X into program and get X synthetic tokens back 
  console.log('staking...')
  await program.rpc.add(new anchor.BN(10), {
    accounts: operation_accounts
  });
  await print_state()
  console.log('')
  
  // wait amount of time 
  console.log('waiting...')
  console.log('')
  await new Promise(r => setTimeout(r, 5000));  

  // revert back to OG state 
  console.log('unstaking...')
  await program.rpc.remove({
    accounts: operation_accounts
  });
  await print_state()

});
