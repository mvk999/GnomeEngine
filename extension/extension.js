import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

export default class GnomeEngineExtension extends Extension {
    enable() {
        console.info('GnomeEngine extension enabled');
    }

    disable() {
        console.info('GnomeEngine extension disabled');
    }
}
