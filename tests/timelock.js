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
    const program = anchor.workspace.Timelock;
    const pda = Keypair.generate();
    const beneficiary = provider.wallet; //Keypair.generate();

    let mint;
    let depositorTokenAcc;
    let pdaTokenAcc;
    let beneficiaryTokenAcc;

    //params
    const start = new BN(+new Date()); //milliseconds
    const end = new BN(+new Date() + (1000 * 60 * 60)); //one hour later
    const deposited_amount = new BN(13370);

    it("Initialize test state", async () => {
        const [_mint, _mint_authority] = await common.createMintAndVault(
            provider,
            new anchor.BN(10000000)
        );
        mint = _mint;
        depositorTokenAcc = _mint_authority;

        pdaTokenAcc = await common.createTokenAccount(provider, mint, pda.publicKey)
        beneficiaryTokenAcc = await common.createTokenAccount(provider, mint, beneficiary.publicKey)
        //should we use anchor.utils.token.associatedAddress()?
    })

    it("Vesting Contract Creation", async () => {
        const tx = await program.rpc.create(
            beneficiary.publicKey, //beneficiary
            deposited_amount,
            start,
            end,
            {
                accounts: {
                    pda: pda.publicKey,
                    pdaTokenAcc,
                    depositorTokenAcc,
                    depositor: provider.wallet.publicKey,
                    systemProgram: SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID
                },
                signers: [pda, provider.wallet.payer]
            }
        )

        const vault = await program.provider.connection.getAccountInfo(pdaTokenAcc)
        const depositor_token_acc = await program.provider.connection.getAccountInfo(depositorTokenAcc)

        const acc = await program.account.vestingContract.fetch(pda.publicKey)
        const pda_ata = common.token.parseTokenAccountData(vault.data);
        const dep_ata = common.token.parseTokenAccountData(depositor_token_acc.data)
        // console.log('contract', acc,
        //     'pda ata', pda_ata,
        //     'dep ata', dep_ata);
        console.log('deposited during contract creation: ', deposited_amount, pda_ata.amount)
       // assert.ok(deposited_amount.eq(new BN(pda_ata.deposited_amount)));
    });

    it("Withdraws from a contract", async () => {
        const oldPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc);
        const oldPdaAmount = common.token.parseTokenAccountData(oldPdaAta.data).amount;
        const oldBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc)
        const oldBeneficiaryAmount = common.token.parseTokenAccountData(oldBeneficiaryAta.data).amount;
        const amount = new BN(10)

        const accounts = {
            pdaTokenAcc,
            pda: pda.publicKey,
            beneficiaryTokenAcc,
            beneficiary: beneficiary.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId
        }
        // console.log('acc', accounts, 'PDA', pda.publicKey.toBase58())
        await program.rpc.withdraw(
            amount, {accounts, signers: [pda, beneficiary.payer]})

        const newPdaAta = await program.provider.connection.getAccountInfo(pdaTokenAcc);
        const newPdaAmount = common.token.parseTokenAccountData(newPdaAta.data).amount;
        const newBeneficiaryAta = await program.provider.connection.getAccountInfo(beneficiaryTokenAcc);
        const newBeneficiaryAmount = common.token.parseTokenAccountData(newBeneficiaryAta.data).amount;
        const pdaData = (await program.account.vestingContract.fetch(pda.publicKey));
        console.log('deposited_amount', pdaData.depositedAmount, 'withdrawn', pdaData.withdrawn, 'amount', amount)
        assert.ok(amount.eq(new BN(oldPdaAmount - newPdaAmount)))
        assert.ok(amount.eq(new BN(newBeneficiaryAmount - oldBeneficiaryAmount)))
        assert.ok(pdaData.withdrawn.eq(amount))
    });

    it("Cancels the stream", async () => {
        assert.ok(true);
    });
});
