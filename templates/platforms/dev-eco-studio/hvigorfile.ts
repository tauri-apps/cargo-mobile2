import { appTasks } from "@ohos/hvigor-ohos-plugin";
import { hvigor, HvigorPlugin, HvigorNode } from "@ohos/hvigor";
import { execSync } from "child_process";
import { resolve } from "path";

export default {
  system: appTasks /* Built-in plugin of Hvigor. It cannot be modified. */,
  plugins: [
    cargoMobilePlugin(),
  ] /* Custom plugin to extend the functionality of Hvigor. */,
};

function cargoMobilePlugin(): HvigorPlugin {
  return {
    pluginId: "cargo-mobile",
    apply(node: HvigorNode) {
      const properties = hvigor.getParameter().getProperties();
      const target = properties.target || "aarch64";
      execSync("cargo", ["open-harmony", "build", "--target", target], {
        cwd: resolve(__dirname, "{{root-dir-rel}}"),
        stdio: "inherit",
        shell: true,
      });
    },
  };
}
