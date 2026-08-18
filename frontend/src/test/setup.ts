import '@testing-library/jest-dom';

// Mock Web Audio / HTMLAudioElement
window.HTMLMediaElement.prototype.play = () => Promise.resolve();
window.HTMLMediaElement.prototype.pause = () => {};
window.HTMLMediaElement.prototype.load = () => {};
