#ifndef SYSTEMTRAY_H
#define SYSTEMTRAY_H

#include <QObject>
#include <QPointer>

class QQuickWindow;
class KStatusNotifierItem;
class QAction;

class SystemTray : public QObject
{
    Q_OBJECT

public:
    explicit SystemTray(QQuickWindow *window, QObject *parent = nullptr);
    ~SystemTray() override;

    Q_INVOKABLE void showNotification(const QString &title, const QString &message);
    Q_INVOKABLE void toggleWindow();
    Q_INVOKABLE void hideWindow();
    Q_INVOKABLE void showWindow();

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private Q_SLOTS:
    void onQuitRequested();

private:
    void setupTray();

    QPointer<QQuickWindow> m_window;
    KStatusNotifierItem *m_trayItem = nullptr;
    QAction *m_toggleAction = nullptr;
    bool m_isQuitting = false;
};

#endif // SYSTEMTRAY_H
