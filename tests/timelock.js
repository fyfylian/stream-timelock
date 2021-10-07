const assert = require('assert')
const anchor = require('@project-serum/anchor');
const common = require('@project-serum/common');
const {
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    Token
} = require('@solana/spl-token')
const {
    PublicKey,
    SYSVAR_RENT_PUBKEY
} = require("@solana/web3.js");
const {
    min
} = require("mocha/lib/reporters");
const {
    utils
} = require("@project-serum/anchor");
const {
    SystemProgram,
    Keypair
} = anchor.web3;
const {
    BN
} = anchor;

// The stream recipient main wallet
const recipient = Keypair.generate();

describe('timelock', () => {
    const provider = anchor.Provider.local(); //todo use env()
    anchor.setProvider(provider);

    const program = anchor.workspace.Timelock;
    const sender = provider.wallet;
    const metadata = Keypair.generate();
    const MINT_DECIMALS = 8;
    let escrowTokens;
    let recipientTokens;
    let nonce;
    let mint;
    let senderTokens;

    // Needed to sign transactions in the name of metadata account during withdrawal/cancel.
    // let escrowSigner; 
    // let escrowCliffSigner;

    // Divide by 1000 since Unix timestamp is seconds
    const start = new BN(+new Date() / 1000 + 1);
    // +60 seconds
    const end = new BN((+new Date()) / 1000 + 60);
    // In seconds
    const period = new BN(3);
    // Amount to deposit
    const depositedAmount = new BN(1337_000_000);
    //const depositedAmount = new BN(133769 * 10 ** MINT_DECIMALS);


    it("Initialize test state", async () => {
        [mint, senderTokens] = await common.createMintAndVault(
            provider,
            new anchor.BN(100_000_000_000),
            undefined,
            MINT_DECIMALS,
        );

        [escrowTokens, nonce] = await PublicKey.findProgramAddress(
            [metadata.publicKey.toBuffer()],
            program.programId);

        recipientTokens = await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            mint,
            recipient.publicKey);

        console.log('associated token program:', ASSOCIATED_TOKEN_PROGRAM_ID.toString())
        console.log('recipient wallet:', recipient.publicKey.toBase58())
        console.log('recipient tokens:', recipientTokens.toBase58())
        console.log('program ID:', program.programId.toBase58())
        console.log('mint:', mint.toBase58())
    })

    it("Create Vesting Contract w/out the cliff", async () => {
        console.log('metadata:', metadata.publicKey.toBase58());
        console.log('buffer:', metadata.publicKey.toBuffer());

        const tx = await program.rpc.create(
            // Order of the parameters must match the ones in the program
            depositedAmount,
            start,
            end,
            period,
            new BN(0), //cliff
            new BN(0), //cliff amount
            {
                accounts: {
                    sender: sender.publicKey,
                    senderTokens,
                    recipient: recipient.publicKey,
                    recipientTokens,
                    metadata: metadata.publicKey,
                    escrowTokens,
                    mint,
                    rent: SYSVAR_RENT_PUBKEY,
                    timelockProgram: program.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
                },
                signers: [metadata, sender.payer],
            });

        const _escrowTokens = await program.provider.connection.getAccountInfo(escrowTokens);
        const _senderTokens = await program.provider.connection.getAccountInfo(senderTokens);

        const _metadata = await program.provider.connection.getAccountInfo(metadata.publicKey);
        const _escrowTokensData = common.token.parseTokenAccountData(_escrowTokens.data);
        const _senderTokensData = common.token.parseTokenAccountData(_senderTokens.data);

        console.log('metadata', _metadata.data,
            'escrow tokens', _escrowTokensData,
            'senderTokens', _senderTokensData);

        console.log('deposited during contract creation: ',
            depositedAmount.toNumber(),
            _escrowTokensData.amount);

        assert.ok(depositedAmount.toNumber() === _escrowTokensData.amount);
    });

    it("Withdraws from a contract", async () => {
        setTimeout(async () => {
            console.log('recipient tokens', recipientTokens.toBase58())
            const oldEscrowAta = await program.provider.connection.getAccountInfo(escrowTokens);
            const oldEscrowAmount = common.token.parseTokenAccountData(oldEscrowAta.data).amount;
            const oldRecipientAta = await program.provider.connection.getAccountInfo(recipientTokens)
            const oldRecipientAmount = common.token.parseTokenAccountData(oldRecipientAta.data).amount;
            const withdrawAmount = new BN(0);

            console.log('metadata', metadata.publicKey.toBase58(), 'escrow_ata', escrowTokens.toBase58())
            console.log('seed', metadata.publicKey.toBuffer())

            console.log('metadata', metadata.publicKey.toBase58())
            await program.rpc.withdraw(withdrawAmount, {
                accounts: {
                    recipient: recipient.publicKey,
                    recipientTokens,
                    metadata: metadata.publicKey,
                    escrowTokens,
                    mint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                }, signers: [recipient]
            })

            const newEscrowAta = await program.provider.connection.getAccountInfo(escrowTokens);

            const newEscrowAmount = common.token.parseTokenAccountData(newEscrowAta.data).amount;
            const newRecipientAta = await program.provider.connection.getAccountInfo(recipientTokens);
            const newRecipientAmount = common.token.parseTokenAccountData(newRecipientAta.data).amount;
            //const escrowData = (await program.account.vestingContract.fetch(metadata.publicKey));

            console.log('depositedAmount', depositedAmount.toNumber(), 'withdrawn', withdrawAmount)
            console.log('old', oldEscrowAmount, 'new', newEscrowAmount)
            console.log('old amount recipient', oldRecipientAmount, 'new amount recipient', newRecipientAmount)
            assert.ok(withdrawAmount.eq(new BN(oldEscrowAmount - newEscrowAmount)))
            assert.ok(withdrawAmount.eq(new BN(newRecipientAmount - oldRecipientAmount)))
            // assert.ok(escrowData.withdrawn.eq(withdrawAmount))
        }, 4500);
    });

    // it("Cancels the stream", async () => {
    //     setTimeout(async () => {
    //         const oldescrowAta = await program.provider.connection.getAccountInfo(escrowTokens.publicKey);
    //         const oldescrowAmount = common.token.parseTokensountData(oldescrowAta.data).amount;
    //         const oldrecipientAta = await program.provider.connection.getAccountInfo(recipientTokens)
    //         const oldrecipientAmount = common.token.parseTokensountData(oldrecipientAta.data).amount;
    //
    //         console.log('escrowsig', escrowSigner, metadata.publicKey)
    //         await program.rpc.cancel({
    //             accounts: {
    //                 metadata: metadata.publicKey,
    //                 escrowTokens: escrowTokens.publicKey,
    //                 escrowSigner,
    //                 recipient: recipient.publicKey,
    //                 recipientTokens,
    //                 sender: sender.publicKey,
    //                 senderTokens,
    //                 tokenProgram: TOKEN_PROGRAM_ID,
    //             }
    //         })
    //
    //         const newescrowAta = await program.provider.connection.getAccountInfo(escrowTokens.publicKey);
    //         const newescrowAmount = common.token.parseTokensountData(newescrowAta.data).amount;
    //         const newrecipientAta = await program.provider.connection.getAccountInfo(recipientTokens);
    //         const newrecipientAmount = common.token.parseTokensountData(newrecipientAta.data).amount;
    //         const escrowData = (await program.account.vestingContract.fetch(metadata.publicKey));
    //         console.log('depositedAmount', escrowData.depositedAmount.toNumber(), 'withdrawn', escrowData.withdrawn.toNumber(), 'recipient amount', newrecipientAmount.toNumber())
    //         assert.ok(newescrowAmount.toNumber() === 0)
    //
    //     }, 9000);
    // });
    //
    // it("Transfers vesting contract ownership", async () => {
    //     const oldrecipient = (await program.account.vestingContract.fetch(metadata.publicKey)).recipient;
    //
    //     await program.rpc.transfer({
    //         accounts: {
    //             metadata: metadata.publicKey,
    //             recipient: recipient.publicKey,
    //             newrecipient: new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")
    //         }
    //     })
    //
    //     const newrecipient = (await program.account.vestingContract.fetch(metadata.publicKey)).recipient;
    //     console.log(oldrecipient.toBase58(), newrecipient.toBase58())
    //     assert.ok(oldrecipient !== newrecipient)
    //     assert.ok(newrecipient.toBase58() === "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")
    // });
});
