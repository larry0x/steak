import Head from "next/head";
import { NextPage } from "next";

import BondForm from "../components/BondForm";

const BondPage: NextPage = () => {
  return (
    <>
      <Head>
        <title>Steak | Bond</title>
      </Head>
      <BondForm />
    </>
  );
};

export default BondPage;
