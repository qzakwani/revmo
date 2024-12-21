<div align="center">
  
![Revmo Logo](https://github.com/qzakwani/revmo/blob/master/resources/icon.png)

# 🚀 **Revmo** 🚀

**Your apps, just a keystroke away.**

Welcome to **Revmo**, the app launcher that understands you’re busy. Say goodbye to endless scrolling and fumbling around for that one app. With Revmo, your apps gather in one elegant window, right in the center of your screen, ready for you to look up and launch.

_**Because clicking is overrated, and efficiency is everything.**_
</div>

### 🧩 **Key Features**

- 🎯 **No Mouse Needed**: Navigate and launch apps effortlessly with just your keyboard.
- 🕒 **Launch History**: Quickly revisit your most frequently or recently launched apps.
- ⚡ **Blazingly Fast**: Powered by Rust for unparalleled performance.
- 📦 **Self-Contained**: Fully standalone with zero dependencies—just download and go.
- 🎨 **Vibrant Themes**: A range of sleek themes to suit your style.
- 🗑️ **Customizable App List**: Remove unwanted apps with ease.

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
curl -fsSL https://raw.githubusercontent.com/qzakwani/revmo/refs/heads/master/scripts/install.sh | sh
```

💥 **Boom!** Revmo will automatically download and install itself—no extra steps required!

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
  Launch **Revmo** instantly by binding it to a keyboard shortcut! Most desktop environments allow you to assign custom shortcuts to applications. Simply bind the `revmo` command to your favorite key combination, like `Alt + Space` or `Super + J`, for seamless access.

- **Use Themes to Match Your Style**  
  Customize **Revmo** to blend with your desktop setup by selecting a theme that complements your workflow.

<div align="center">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/theme1.png" alt="theme" width="250">
  <img src="https://github.com/qzakwani/revmo/blob/master/resources/theme2.png" alt="theme" width="250">
</div>

- **Take Advantage of App History**  
  Revmo keeps a history of your launched apps, so you can quickly re-open frequently used applications without retyping.

- **Clean Up Your App List**  
  Found unwanted apps in your launcher? Remove them from the list to keep your search results clean and focused.

With these simple tweaks, you can elevate your productivity and enjoy the full power of **Revmo**! 🚀

### **4. Uninstallation**

If you ever need to remove **Revmo**, the process is just as simple as installing it. Run the following command in your terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/qzakwani/revmo/refs/heads/master/scripts/uninstall.sh | sh
```

🧹 **Clean Exit**: This will completely remove Revmo from your system, leaving no traces behind.

### **5. Potential Problems and Solutions**

Here are some common issues you might encounter and how to fix them:

- **Revmo Doesn't Launch**

  - **Cause**: The `revmo` command might not be in your system's PATH.
  - **Solution**: If you used the installation method above, add `revmo` to your PATH, or the whole dir: `$HOME/.local/bin` (should be there by default!)

- **Shortcut Key Doesn’t Work**
  - **Cause**: Desktop environment settings may not have applied the custom shortcut or the command is not found.
  - **Solution**: Double-check your keybinding settings and reassign the shortcut or add the full command path: `$HOME/.local/bin/revmo` -> change home with yours (e.g., /home/foo/.local/bin/revmo)

If you encounter other problems, feel free file an issue. I’ve got you covered! 🛠️

