<div align="center">
  
![Revmo Logo](https://github.com/qzakwani/revmo/blob/master/resources/icon.png)

# 🚀 **Revmo** 🚀

**Your apps, just a keystroke away.**

Welcome to **Revmo**, the app launcher that understands you’re busy. Say goodbye to endless scrolling and fumbling around for that one app. With Revmo, your apps gather in one elegant window, right in the center of your screen, ready for you to look up and launch.

_**Because clicking is overrated, and efficiency is everything.**_
</div>

### 🧩 **Key Features**

- 🎯 **No Mouse Needed**: Navigate and launch apps effortlessly with just your keyboard.
- 🔎 **Fuzzy Search**: Type a few letters (`lw` → LibreOffice Writer); app descriptions and keywords are searched too.
- 🕒 **Launch History**: Your most frequently and recently launched apps come first.
- ⚡ **Blazingly Fast**: Powered by Rust for unparalleled performance.
- 📦 **Self-Contained**: Fully standalone with zero dependencies—just download and go.
- 🎨 **Themes**: 24 color themes, or write your own in a few lines.
- 🗑️ **Customizable App List**: Hide unwanted apps with one line in the config file.

<div align="center">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/app.png" alt="app" width="45%">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/app_screen.png" alt="app in action" width="45%">
</div>

## 🛠️ **Getting Started**

Before you install, make sure your system meets the following requirements:

- **🖥️ OS**: Linux
- **💻 Architecture**: x86_64
- **⚙️ Needs**: A GNU system and a desktop environment (e.g., Hyprland, GNOME, KDE Plasma, Cinnamon)


### **1. Installation**

To install Revmo, simply run the following curl command:

```sh
curl -fsSL https://raw.githubusercontent.com/qzakwani/revmo/refs/heads/main/scripts/install.sh | sh
```

💥 **Boom!** Revmo will automatically download and install itself—no extra steps required!

To update Revmo, run the same command again: it installs the latest release over the old one and keeps your settings, themes and history.

### **2. Running Revmo**

Once installed, you can easily launch **Revmo** in two ways:

 - **CLI Command**:
   Simply run the following command in your terminal:
   ```sh
   revmo
   ```

 - **Application Menu**:  
    Alternatively, you can find **Revmo** in your application menu or launcher (depending on your desktop environment).
   Click the **Revmo** icon to launch it.

🚀 **Ready to Go!** Once launched, **Revmo** will appear in the center of your screen, ready to help you launch your favorite apps!

### **3. Tips for Using Revmo Effectively**

To make the most out of **Revmo**, consider these tips:

- **Bind Revmo to a Keyboard Shortcut**  
  Launch **Revmo** instantly by binding it to a keyboard shortcut! Most desktop environments allow you to assign custom shortcuts to applications. Simply bind the `revmo` command to your favorite key combination, like `Alt + Space` or `Super + J`, for seamless access. Pressing the shortcut again (or clicking outside the window) closes Revmo.

- **Use Themes to Match Your Style**  
  Press `Ctrl + →` / `Ctrl + ←` to cycle through the themes. Revmo Dark and Revmo Light are built in; the installer adds 22 more to `~/.local/share/revmo/themes`. To make your own, create `~/.config/revmo/themes/<name>.theme`:

  ```ini
  [Theme]
  Name=Mine
  Background=#1d2021
  Text=#ebdbb2
  Accent=#b8bb26
  ```

  `Background`, `Text` and `Accent` are required. Revmo works out the other colors from them, or you can set any of `Raised`, `SelectedSoft`, `Frame`, `Border`, `BorderStrong`, `TextDim`, `TextMuted`, `TextFaint`, `OnAccent`, `Selection`, `OnSelection`, `Danger`, `Success` and `Highlight` (matched letters) yourself (`#rrggbb` or `#rrggbbaa`). A theme in `~/.config` replaces an installed one with the same name. Press `Ctrl + R` to load new or edited themes.

- **The Config File**  
  All settings live in one plain-text file, `~/.config/revmo/config`, that you write yourself (and can keep with your dotfiles, symlinked or not). You don't need one: every setting you leave out uses its default.

  ```ini
  [Revmo]
  # Also changed by Ctrl + ← / Ctrl + →
  Theme=Revmo Dark
  # Empty: the bundled IBM Plex Mono
  Font=
  # Empty or auto: picked from your monitor
  Scale=
  # Apps to leave out, see below
  Hidden=
  ```

  Comments go on their own lines starting with `#`. `Theme=` is the only setting Revmo writes: when `Ctrl + ←` / `Ctrl + →` saves a theme, only that line changes (or is added, creating the file if there's none) and the rest is kept as you wrote it.

  If a line has a mistake (an unknown setting, a theme or font that isn't installed, a scale that isn't a number), Revmo uses the default for that setting and shows the problem when it opens; `Ctrl + H` lists them all. If Revmo can't read or understand the file at all, it leaves it untouched and won't save theme changes to it.

- **Pick Your Font and Size**  
  Revmo comes with IBM Plex Mono. To use another installed font, set `Font=` in the config file to its family name (e.g. `Font=JetBrains Mono`). Run `revmo list-fonts` to see the names you can use; fonts marked `mono` (monospaced) suit Revmo's look best.

  If Revmo looks too small or too large, set `Scale=` in the config file (e.g. `Scale=1.5`); it scales the window and text together. Left empty, Revmo picks a scale from your monitor's size on X11 (a 27" 4K screen at 96 dpi gets 1.5, sized by the primary monitor) and follows the desktop's scaling on Wayland. If the automatic scale is wrong, for example with several monitors, set `Scale=` yourself. The help page (`Ctrl + H`) shows the scale in use. Font and scale changes apply the next time Revmo opens.

<div align="center">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/theme1.png" alt="theme" width="250">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/theme2.png" alt="theme" width="250">
</div>

- **Take Advantage of App History**  
  Revmo keeps a history of your launched apps, so you can quickly re-open frequently used applications without retyping.

- **Actions, Files and Links**  
  Each app has one row. Press `→` to open its actions pane: open it, or use one of its actions (e.g. *New Private Window*). Entries that accept files or URLs show `→ file`/`→ url`; press `→` on one to type the files to open with it, and wrap names containing spaces in quotes when giving several.

- **Clean Up Your App List**  
  To keep apps out of Revmo, list their IDs in the config file, separated by `;`. Run `revmo list-apps` to see every app and action with its ID on the right:

  ```console
  $ revmo list-apps | grep -i logs
  Logs                  org.gnome.Logs.desktop
  ```

  ```ini
  Hidden=org.gnome.Logs.desktop;nm-connection-editor.desktop;
  ```

  To hide only one action, use its ID (e.g. `firefox.desktop/new-private-window`). A hidden app's actions are hidden with it, and `revmo list-apps` marks what's hidden, so you can find what to remove to bring an app back. To hide an app from every launcher, not just Revmo, copy its `.desktop` file to `~/.local/share/applications/` and add `NoDisplay=true`.

- **Keyboard Shortcuts**

  | Key | Action |
  |---|---|
  | Type | Search |
  | `↑` / `↓` / `Tab` | Move the selection |
  | `Enter` | Launch the selected app or action |
  | `→` | Open the actions pane; on an action that takes files, enter them |
  | `←` | Back |
  | `Ctrl + →` / `Ctrl + ←` | Next / previous theme |
  | `Ctrl + R` | Reload the config file and rescan themes and apps (history is kept) |
  | `Ctrl + H` | Help |
  | `Esc` | Back, or close |

- **Command Line**

  | Command | What it does |
  |---|---|
  | `revmo` | Open Revmo (or close it if it's open) |
  | `revmo list-apps` | List apps and their actions, with the IDs `Hidden=` takes |
  | `revmo list-fonts` | List installed font families, as `Font=` takes them |
  | `revmo --help` | Show the commands |
  | `revmo --version` | Show the version |

- **Where Revmo Keeps Its Files**
  - `~/.config/revmo/config`: theme, font, scale and hidden apps (written by you; Revmo only updates `Theme=`)
  - `~/.config/revmo/themes/`: your own themes
  - `~/.local/share/revmo/themes/`: the themes installed with Revmo (replaced on update)
  - `~/.local/state/revmo/usage`: launch history
  - `~/.cache/revmo/apps`, `~/.cache/revmo/themes`: cached apps and themes, rebuilt automatically when they change

With these simple tweaks, you can elevate your productivity and enjoy the full power of **Revmo**! 🚀

### **4. Uninstallation**

If you ever need to remove **Revmo**, the process is just as simple as installing it. Run the following command in your terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/qzakwani/revmo/refs/heads/main/scripts/uninstall.sh | sh
```

🧹 **Clean Exit**: This will completely remove Revmo from your system, leaving no traces behind.

### **5. Potential Problems and Solutions**

Here are some common issues you might encounter and how to fix them:

- **Revmo Doesn't Launch**

  - **Cause**: The `revmo` command might not be in your system's PATH.
  - **Solution**: If you used the installation method above, add `revmo` to your PATH, or the whole dir: `$HOME/.local/bin` (should be there by default!)

- **A Newly Installed App or Icon Theme Doesn't Show Up**
  - **Solution**: New and removed apps are picked up automatically on the next start. After changing your icon theme, press `Ctrl + R` to rescan.

- **Terminal Apps (e.g. Vim, htop) Don't Open**
  - **Cause**: Revmo opens them in a terminal emulator and didn't find one.
  - **Solution**: Set the `TERMINAL` environment variable to your terminal (e.g. `export TERMINAL=kitty`), or install one of kitty, foot, alacritty, wezterm, ghostty, gnome-terminal, konsole, xfce4-terminal, tilix or xterm.

- **Shortcut Key Doesn’t Work**
  - **Cause**: Desktop environment settings may not have applied the custom shortcut or the command is not found.
  - **Solution**: Double-check your keybinding settings and reassign the shortcut or add the full command path: `$HOME/.local/bin/revmo` -> change home with yours (e.g., /home/foo/.local/bin/revmo)

If you encounter other problems, feel free file an issue. I’ve got you covered! 🛠️

