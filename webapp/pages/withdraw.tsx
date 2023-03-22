import Head from "next/head";
import { NextPage } from "next";

import WithdrawForm from "../components/WithdrawForm";

const WithdrawPage: NextPage = () => {
  return (
    <>
      <Head>
        <title>Steak | Withdraw Unbonded</title>
      </Head>
      <WithdrawForm />
    </>
  );
};

export default WithdrawPage;
