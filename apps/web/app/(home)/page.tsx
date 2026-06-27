import { Metadata } from "next";
import HomePage from "@/components/homePage";

export const metadata: Metadata = {
  title: "Gitlu",
  description: "Gitlu - A Git client",
  icons: new URL("/favicon.ico", "https://Gitlu.app"),
};

export default function Page() {
  return <HomePage />;
}
