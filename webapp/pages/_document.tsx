import Document, { Html, Head, Main, NextScript } from "next/document";

class CustomDocument extends Document {
  override render() {
    return (
      <Html>
        <Head>
          <link rel="preconnect" href="https://fonts.googleapis.com" />
          <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="" />
          <link
            href="https://fonts.googleapis.com/css2?family=Urbanist:wght@600;800&display=swap"
            rel="stylesheet"
          />
          <link rel="shortcut icon" type="image/x-icon" href="/favicon.ico"></link>
          <style>
            {`
              body {
                background-color: #f5d9c0 !important;
              }
              @keyframes wiggle {
                0%   { transform: rotate(0deg);  }
                80%  { transform: rotate(0deg);  }
                85%  { transform: rotate(5deg);  }
                95%  { transform: rotate(-5deg); }
                100% { transform: rotate(0deg);  }
              }
            `}
          </style>
        </Head>
        <body>
          <Main />
          <NextScript />
        </body>
      </Html>
    );
  }
}

export default CustomDocument;
