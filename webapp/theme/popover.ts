const styles = {
  baseStyle: {
    content: {
      border: "none",
      bg: "brand.lighterBrown",
      width: "full",
      py: [5, 6],
      px: 6,
      boxShadow: "xl",
      borderRadius: "2xl",
      _focus: {
        boxShadow: "none",
      },
      // https://github.com/chakra-ui/chakra-ui/issues/3553#issuecomment-843043883
      "&:focus:not([data-focus-visible-added])": {
        shadow: "xl", // default shadow
      },
    },
    header: {
      borderBottomWidth: 0,
      fontSize: "2xl",
      p: 0,
    },
    body: {
      p: 0,
    },
    popper: {
      zIndex: 9999,
    },
  },
  sizes: {
    xs: {
      popper: {
        maxWidth: "xs",
      },
    },
  },
  defaultProps: {
    flip: true,
  },
};

export default styles;
