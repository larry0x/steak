import { chakra } from "@chakra-ui/react";
import NextLink  from "next/link";
import { useRouter } from "next/router";
import { FC, MouseEventHandler } from "react";
import * as csstype from "csstype";

type Props = {
  text: string;
  href: string;
  onClick?: MouseEventHandler<HTMLAnchorElement>;
  underConstruction?: boolean;
};

const NavbarLink: FC<Props> = ({ text, href, onClick, underConstruction = false }) => {
  const { asPath } = useRouter();

  const defaultStyle = {
    fontSize: "24px",
    fontWeight: 800,
  };

  const wrapperStyle = underConstruction
    ? {
        color: "rgba(210, 187, 166, 0.8)",
        pointerEvents: "none" as csstype.Property.PointerEvents, // need this or type error
      }
    : asPath === href
    ? {
        color: "brand.red",
        textDecoration: "underline",
        textUnderlineOffset: "8px",
        textDecorationThickness: "3.5px",
      }
    : {
        color: "brand.darkerBrown",
        _hover: {
          color: "brand.red",
        },
      };

  return (
    <NextLink href={href} passHref>
      <chakra.a
        transition="0.2s all"
        p="2"
        whiteSpace="nowrap"
        onClick={onClick}
        {...defaultStyle}
        {...wrapperStyle}
      >
        {text}
        {underConstruction ? <sup>(soon)</sup> : null}
      </chakra.a>
    </NextLink>
  );
};

export default NavbarLink;
