// ===== VIEW STATE MANAGER =====
// Centralized management of app view states
export const ViewManager = {
    closeAll() {
        document.body.classList.remove('detail-open', 'settings-open');
    },

    showRecordings() {
        this.closeAll();
    },

    showDetail() {
        this.closeAll();
        document.body.classList.add('detail-open');
    },

    showSettings() {
        this.closeAll();
        document.body.classList.add('settings-open');
    }
};
