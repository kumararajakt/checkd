#include "systemtray.h"

#include <QApplication>
#include <QCloseEvent>
#include <QIcon>
#include <QMenu>
#include <QQuickWindow>
#include <KLocalizedString>
#include <KStatusNotifierItem>

SystemTray::SystemTray(QQuickWindow *window, QObject *parent)
    : QObject(parent)
    , m_window(window)
{
    setupTray();

    if (m_window) {
        m_window->installEventFilter(this);
    }
}

SystemTray::~SystemTray() = default;

void SystemTray::setupTray()
{
    m_trayItem = new KStatusNotifierItem(QStringLiteral("checkd"), this);
    m_trayItem->setCategory(KStatusNotifierItem::ApplicationStatus);
    m_trayItem->setStatus(KStatusNotifierItem::Active);
    m_trayItem->setTitle(QStringLiteral("CheckD"));
    m_trayItem->setToolTipTitle(QStringLiteral("CheckD"));
    m_trayItem->setToolTipSubTitle(i18n("Task & Todo Manager"));

    // Prefer checkd icon, falling back to standard theme icons
    if (QIcon::hasThemeIcon(QStringLiteral("checkd"))) {
        m_trayItem->setIconByName(QStringLiteral("checkd"));
    } else if (QIcon::hasThemeIcon(QStringLiteral("checkbox"))) {
        m_trayItem->setIconByName(QStringLiteral("checkbox"));
    } else {
        m_trayItem->setIconByName(QStringLiteral("view-list-symbolic"));
    }

    if (m_window) {
        m_trayItem->setAssociatedWindow(m_window);
    }

    // Configure context menu
    QMenu *menu = m_trayItem->contextMenu();
    if (menu) {
        m_toggleAction = new QAction(i18n("Open CheckD"), this);
        m_toggleAction->setIcon(QIcon::fromTheme(QStringLiteral("window-duplicate")));
        connect(m_toggleAction, &QAction::triggered, this, &SystemTray::toggleWindow);

        if (!menu->actions().isEmpty()) {
            menu->insertAction(menu->actions().first(), m_toggleAction);
            menu->insertSeparator(menu->actions().at(1));
        } else {
            menu->addAction(m_toggleAction);
        }
    }

    connect(m_trayItem, &KStatusNotifierItem::quitRequested, this, &SystemTray::onQuitRequested);

    // Update toggle action text dynamically based on window visibility
    if (m_window) {
        connect(m_window, &QWindow::visibleChanged, this, [this](bool visible) {
            if (m_toggleAction) {
                m_toggleAction->setText(visible ? i18n("Hide CheckD") : i18n("Open CheckD"));
            }
        });
    }
}

void SystemTray::toggleWindow()
{
    if (!m_window) {
        return;
    }

    if (m_window->isVisible()) {
        hideWindow();
    } else {
        showWindow();
    }
}

void SystemTray::hideWindow()
{
    if (m_trayItem) {
        m_trayItem->hideAssociatedWindow();
    } else if (m_window) {
        m_window->hide();
    }
}

void SystemTray::showWindow()
{
    if (m_trayItem) {
        m_trayItem->activate();
    } else if (m_window) {
        m_window->show();
        m_window->raise();
        m_window->requestActivate();
    }
}

void SystemTray::showNotification(const QString &title, const QString &message)
{
    if (m_trayItem) {
        m_trayItem->showMessage(title, message, m_trayItem->iconName(), 5000);
    }
}

void SystemTray::onQuitRequested()
{
    m_isQuitting = true;
    qApp->quit();
}

bool SystemTray::eventFilter(QObject *watched, QEvent *event)
{
    if (watched == m_window && event->type() == QEvent::Close) {
        if (!m_isQuitting) {
            event->ignore();
            hideWindow();
            return true;
        }
    }
    return QObject::eventFilter(watched, event);
}
