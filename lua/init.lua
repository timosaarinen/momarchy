-- Momarchy Home configuration.
--
-- This file is compiled into the Momarchy binary and written to
-- ~/.config/momarchy/init.lua on first run. After that, the user's copy is
-- authoritative and can be edited live without rebuilding Momarchy.

local ui = require("momarchy.ui")

return ui.app {
  home = "home",

  theme = {
    layout = {
      columns = 2,
      gap = 1,
      margin = 1,
    },

    colors = {
      background = "black",
      text = "white",
      muted = "gray",
      selected_background = "white",
      selected_text = "black",
    },

    border = "rounded",
  },

  screens = {
    home = ui.screen {
      ui.title "MOMARCHY",
      ui.subtitle "Mitä haluat tehdä?",

      ui.button(
        "internet",
        "INTERNET",
        "Avaa selain",
        ui.open("https://www.google.fi/", "Avataan internet.")
      ),

      ui.button(
        "email",
        "SÄHKÖPOSTI",
        "Lue ja lähetä viestejä",
        ui.message "Sähköposti otetaan käyttöön seuraavaksi."
      ),

      ui.button(
        "photos",
        "KUVAT",
        "Katso kuvia",
        ui.message "Kuvat otetaan käyttöön seuraavaksi."
      ),

      ui.button(
        "youtube",
        "YOUTUBE",
        "Katso videoita",
        ui.open("https://www.youtube.com/", "Avataan YouTube.")
      ),

      ui.button(
        "ask",
        "KYSY MITÄ VAIN",
        "Kirjoita tai puhu kysymys",
        ui.message "Kysy mitä vain tulee seuraavaksi."
      ),

      ui.button(
        "tv",
        "KATSO TELEVISIOSTA",
        "Chromecast",
        ui.message "Chromecast-tuki tulee seuraavaksi."
      ),

      ui.button("games", "PELIT", "Palikat, Mato...", ui.go "games"),
      ui.button("help", "APUA", "Jos jokin ei toimi", ui.go "help"),
    },

    games = ui.screen {
      ui.title "PELIT",
      ui.subtitle "Valitse peli",

      ui.button(
        "palikat",
        "PALIKAT",
        "Putoavia palikoita",
        ui.message "Palikat tulee pian :)"
      ),
      ui.button("mato", "MATO", "Syö ja kasva", ui.message "Mato tulee pian :)"),
      ui.button("back", "TAKAISIN", "Palaa alkuun", ui.go "home"),
    },

    help = ui.screen {
      ui.title "APUA",
      ui.subtitle "Jos jokin ei toimi",
      ui.text [[
Momarchy tarkistaa myöhemmin tästä internet-yhteyden ja muut tärkeät asiat.

Jos jokin ei toimi, pyydä apua.
      ]],

      ui.button("back", "TAKAISIN", "Palaa alkuun", ui.go "home"),
    },
  },
}
