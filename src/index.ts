// Add subtle animations on page load
document.addEventListener('DOMContentLoaded', (): void => {
    const cards = document.querySelectorAll<HTMLAnchorElement>('.mode-card');
    cards.forEach((card: HTMLAnchorElement, index: number): void => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(20px)';

        setTimeout((): void => {
            card.style.transition = 'all 0.6s ease';
            card.style.opacity = '1';
            card.style.transform = 'translateY(0)';
        }, 200 * index);
    });
});

// Add click analytics (placeholder)
document.querySelectorAll<HTMLAnchorElement>('.mode-card').forEach((card: HTMLAnchorElement): void => {
    card.addEventListener('click', (e: MouseEvent): void => {
        const modeTitle = card.querySelector<HTMLElement>('.mode-title');
        if (modeTitle) {
            const mode = modeTitle.textContent;
            console.log(`User selected: ${mode} mode`);
        }
    });
});
