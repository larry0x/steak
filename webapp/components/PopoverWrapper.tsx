import React, { FC, ReactNode } from "react";
import {
  Flex,
  Popover,
  PopoverContent,
  PopoverHeader,
  PopoverCloseButton,
  PopoverBody,
  PopoverTrigger,
  PopoverProps,
} from "@chakra-ui/react";

import CloseIcon from "./CloseIcon";

type Props = {
  title?: string;
  triggerElement: () => React.ReactElement;
  children: ReactNode;
} & PopoverProps;

const PopoverWrapper: FC<Props> = ({ title, triggerElement, children, ...props }) => {
  return (
    <Popover {...props}>
      <PopoverTrigger>{triggerElement()}</PopoverTrigger>
      <PopoverContent px="10">
        <Flex align="center" justify="space-between">
          <PopoverHeader>{title}</PopoverHeader>
          <PopoverCloseButton position="static" width="2rem" height="2rem" borderRadius="full">
            <CloseIcon w="2rem" h="2rem" />
          </PopoverCloseButton>
        </Flex>
        <PopoverBody w="100%">{children}</PopoverBody>
      </PopoverContent>
    </Popover>
  );
};

export default PopoverWrapper;
