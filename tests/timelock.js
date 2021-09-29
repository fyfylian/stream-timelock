const assert = require('assert')
const anchor = require('@project-serum/anchor');
const common = require('@project-serum/common');
const {TOKEN_PROGRAM_ID} = require('@solana/spl-token')
const {SystemProgram, Keypair} = anchor.web3;
const {BN} = anchor;

describe('timelock', () => {
    const provider = anchor.Provider.local();//todo use env()
    anchor.setProvider(provider);

    //accounts
    const depositor = provider.wallet;
    const program = anchor.workspace.Timelock;
    const pda = Keypair.generate();
    const pdaTokenAcc = Keypair.generate();
    const beneficiary = provider.wallet; //todo update to something else, e.g. known address or Keypair.generate();

    let mint;
    let depositorTokenAcc;
    let beneficiaryTokenAcc;

    let pdaSigner; //needed to sign transactions in the name of PDA account during withdrawal/cancel.

    const start = new BN(+new Date() / 1000 + 5); //divide by 1000 since unix timestamp is in seconds and add 5 seconds
    const end = new BN((+new Date()) / 1000 + 60); //one min later
    const deposited_amount = new BN(1337);
    let withdrawAmount;

    it("Initialize test state", async () => {
        const [_mint, _mint_authority] = await common.createMintAndVault(
            provider,
            new anchor.BN(10_000)
        );
        mint = _mint;
        depositorTokenAcc = _mint_authority;

        // must check if the associated token account already exists
        // pdaTokenAcc = await common.createTokenAccount(provider, mint, pda.publicKey)
        // todo ne kreira se ovde
        // beneficiaryTokenAcc = await common.createTokenAccount(provider, mint, beneficiary.publicKey)
        // should we use anchor.utils.token.associatedAddress()?
    })

    it("Vesting Contract Creation", async () => {
        //if seeds are known, signer can be derived.
        let [_pdaSigner, nonce] = await anchor.web3.PublicKey.findProgramAddress(
            [pda.publicKey.toBuffer()],//(Seeds can be anything but we decided those will be serialized pda's pubkey
            program.programId
        );

        pdaSigner = _pdaSigner;
        console.log('pdasig', pdaSigner.toBase58(), 'pda', pda.publicKey.toBase58(), 'buffer', pda.publicKey.toBuffer())
        console.log('nonce', nonce);

        const tx = await program.rpc.create(
            //order of the parameters must match the ones in program
            beneficiary.publicKey, //beneficiary
            deposited_amount,
            start,
            end,
            nonce,
            {
                accounts: {
                    pda: pda.publicKey,
                    pdaSigner,
                    pdaTokenAcc: pdaTokenAcc.publicKey,
                    depositorTokenAcc,
                    depositor: depositor.publicKey,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID
                },
                signers: [pda, pdaTokenAcc],
                instructions: [
                    // await program.account.vestingContract.createInstruction(pda),
                    ...(await common.createTokenAccountInstrs(provider, pdaTokenAcc.publicKey, mint, pdaSigner))
                ]
            },
        )

        const vault = await program.provider.connection.getAccountInfo(pdaTokenAcc.publicKey)
        const depositor_token_acc = await program.provider.connection.getAccountInfo(depositorTokenAcc)

        const acc = await program.account.vestingContract.fetch(pda.publicKey)
        const pda_ata = common.token.parseTokenAccountData(vault.data);
        const dep_ata = common.token.parseTokenAccountData(depositor_token_acc.data)
        // console.log('contract', acc,
        //     'pda ata', pda_ata,
        //     'dep ata', dep_ata);
        console.log('deposited during contract creation: ', deposited_amount.toNumber(), pda_ata.amount)
        assert.ok(deposited_amount.toNumber() === pda_ata.amount);
    });

    it("Withdraws from a contract", async () => {
        setTimeout(async () => {
            // beneficiaryTokenAcc = await utils.token.associatedAddress({mint, owner: beneficiary.publicKey})
            //console.log('beneficiary ata', beneficiaryTokenAcc.toBase58())
            beneficiaryTokenAcc = await common.createTokenAccount(provider, mint, beneficiary.publicKey);
            console.log('beneficiary ata 2', beneficiaryTokenAcc.toBase58())

            const oldPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc.publicKey);
            const oldPdaAmount = common.token.parseTokenAccountData(oldPdaAta.data).amount;
            const oldBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc)
            const oldBeneficiaryAmount = common.token.parseTokenAccountData(oldBeneficiaryAta.data).amount;
            const withdrawAmount = new BN(10);

            console.log('pda sig', pdaSigner.toBase58(), 'pda', pda.publicKey.toBase58(), 'pda_ata_client', pdaTokenAcc.publicKey.toBase58(), 'pda_tok_acc', (await program.account.vestingContract.fetch(pda.publicKey)).pdaTokenAcc.toBase58())
            console.log('seed', pda.publicKey.toBuffer())
            const accounts = {
                pda: pda.publicKey,
                pdaTokenAcc: pdaTokenAcc.publicKey,
                pdaSigner,
                beneficiaryTokenAcc,
                beneficiary: beneficiary.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId
            }

            console.log('acc', accounts, 'PDA', pda.publicKey.toBase58())
            await program.rpc.withdraw(withdrawAmount, {accounts})

            const newPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc.publicKey);
            const newPdaAmount = common.token.parseTokenAccountData(newPdaAta.data).amount;
            const newBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc);
            const newBeneficiaryAmount = common.token.parseTokenAccountData(newBeneficiaryAta.data).amount;
            const pdaData = (await program.account.vestingContract.fetch(pda.publicKey));
            console.log('deposited_amount', pdaData.depositedAmount, 'withdrawn', pdaData.withdrawn, 'amount', withdrawAmount)
            assert.ok(withdrawAmount.eq(new BN(oldPdaAmount - newPdaAmount)))
            assert.ok(withdrawAmount.eq(new BN(newBeneficiaryAmount - oldBeneficiaryAmount)))
            assert.ok(pdaData.withdrawn.eq(withdrawAmount))
        }, 6000);
    });

    it("Cancels the stream", async () => {
        setTimeout(async () => {
            const oldPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc.publicKey);
            const oldPdaAmount = common.token.parseTokenAccountData(oldPdaAta.data).amount;
            const oldBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc)
            const oldBeneficiaryAmount = common.token.parseTokenAccountData(oldBeneficiaryAta.data).amount;

            console.log('pdasig', pdaSigner, pda.publicKey)
            await program.rpc.cancel({
                accounts: {
                    pda: pda.publicKey,
                    pdaTokenAcc: pdaTokenAcc.publicKey,
                    pdaSigner,
                    beneficiary: beneficiary.publicKey,
                    beneficiaryTokenAcc,
                    depositor: depositor.publicKey,
                    depositorTokenAcc,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                }
            })

            const newPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc.publicKey);
            const newPdaAmount = common.token.parseTokenAccountData(newPdaAta.data).amount;
            const newBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc);
            const newBeneficiaryAmount = common.token.parseTokenAccountData(newBeneficiaryAta.data).amount;
            const pdaData = (await program.account.vestingContract.fetch(pda.publicKey));
            console.log('deposited_amount', pdaData.depositedAmount.toNumber(), 'withdrawn', pdaData.withdrawn.toNumber(), 'beneficiary amount', newBeneficiaryAmount.toNumber())
            assert.ok(newPdaAmount.toNumber() === 0)

        }, 9000);
    });
});
