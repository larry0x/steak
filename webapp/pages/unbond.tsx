import Head from "next/head";
import { NextPage } from "next";

import UnbondForm from "../components/UnbondForm";

const UnbondPage: NextPage = () => {
  return (
    <>
      <Head>
        <title>Steak | Unbond</title>
      </Head>
      <UnbondForm />
    </>
  );
};

export default UnbondPage;
