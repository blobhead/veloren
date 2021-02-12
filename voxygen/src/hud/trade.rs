use super::{
    img_ids::Imgs,
    item_imgs::ItemImgs,
    slots::{SlotManager, TradeSlot},
    TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        TooltipManager,
    },
};
use client::Client;
use common::{
    comp::Inventory,
    trade::{PendingTrade, TradeActionMsg},
};
use common_net::sync::WorldSyncExt;
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Image, Rectangle, State as ConrodState, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use vek::*;

pub struct State {
    ids: Ids,
}

widget_ids! {
    pub struct Ids {
        trade_close,
        bg,
        bg_frame,
        trade_title_bg,
        trade_title,
        inv_alignment[],
        inv_slots[],
        inv_textslots[],
        offer_headers[],
        accept_indicators[],
        phase_indicator,
        accept_button,
        decline_button,
    }
}

#[derive(WidgetCommon)]
pub struct Trade<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    //tooltip_manager: &'a mut TooltipManager,
    slot_manager: &'a mut SlotManager,
    localized_strings: &'a Localization,
}

impl<'a> Trade<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        _tooltip_manager: &'a mut TooltipManager,
        slot_manager: &'a mut SlotManager,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            client,
            imgs,
            item_imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            //tooltip_manager,
            slot_manager,
            localized_strings,
        }
    }
}
const MAX_TRADE_SLOTS: usize = 16;

impl<'a> Trade<'a> {
    fn background(&mut self, state: &mut ConrodState<'_, State>, ui: &mut UiCell<'_>) {
        Image::new(self.imgs.inv_bg_bag)
            .w_h(424.0, 708.0)
            .middle()
            .color(Some(UI_MAIN))
            .set(state.ids.bg, ui);
        Image::new(self.imgs.inv_frame_bag)
            .w_h(424.0, 708.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.bg_frame, ui);
    }

    fn title(&mut self, state: &mut ConrodState<'_, State>, ui: &mut UiCell<'_>) {
        Text::new(&self.localized_strings.get("hud.trade.trade_window"))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.trade_title_bg, ui);
        Text::new(&self.localized_strings.get("hud.trade.trade_window"))
            .top_left_with_margins_on(state.ids.trade_title_bg, 2.0, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.trade_title, ui);
    }

    fn phase_indicator(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
    ) {
        let phase_text = if trade.in_phase1() {
            self.localized_strings.get("hud.trade.phase1_description")
        } else if trade.in_phase2() {
            self.localized_strings.get("hud.trade.phase2_description")
        } else {
            self.localized_strings.get("hud.trade.phase3_description")
        };

        Text::new(&phase_text)
            .mid_top_with_margin_on(state.ids.bg, 70.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
            .set(state.ids.phase_indicator, ui);
    }

    fn item_pane(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
        who: usize,
    ) -> <Self as Widget>::Event {
        let inventories = self.client.inventories();
        let uid = trade.parties[who];
        let entity = self.client.state().ecs().entity_from_uid(uid.0)?;
        let inventory = inventories.get(entity)?;

        // Alignment for Grid
        let mut alignment = Rectangle::fill_with([200.0, 340.0], color::TRANSPARENT);
        if who % 2 == 0 {
            alignment = alignment.top_left_with_margins_on(state.ids.bg, 180.0, 46.5);
        } else {
            alignment = alignment.right_from(state.ids.inv_alignment[0], 0.0);
        }
        alignment
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment[who], ui);

        let name = self
            .client
            .player_list()
            .get(&uid)
            .map(|info| info.player_alias.clone())
            .unwrap_or_else(|| format!("Player {}", who));

        let offer_header = self
            .localized_strings
            .get("hud.trade.persons_offer")
            .replace("{playername}", &name);
        Text::new(&offer_header)
            .up_from(state.ids.inv_alignment[who], 20.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
            .set(state.ids.offer_headers[who], ui);

        let has_accepted = (trade.in_phase1() && trade.phase1_accepts[who])
            || (trade.in_phase2() && trade.phase2_accepts[who]);
        let accept_indicator = self
            .localized_strings
            .get("hud.trade.has_accepted")
            .replace("{playername}", &name);
        Text::new(&accept_indicator)
            .down_from(state.ids.inv_alignment[who], 20.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(
                1.0,
                1.0,
                1.0,
                if has_accepted { 1.0 } else { 0.0 },
            ))
            .set(state.ids.accept_indicators[who], ui);

        let mut invslots: Vec<_> = trade.offers[who].iter().map(|(k, v)| (*k, *v)).collect();
        invslots.sort();
        let tradeslots: Vec<_> = invslots
            .into_iter()
            .enumerate()
            .map(|(index, (k, quantity))| TradeSlot {
                index,
                quantity,
                invslot: Some(k),
            })
            .collect();

        if trade.in_phase1() {
            self.phase1_itemwidget(state, ui, inventory, who, &tradeslots);
        } else {
            self.phase2_itemwidget(state, ui, inventory, who, &tradeslots);
        }

        None
    }

    fn phase1_itemwidget(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        inventory: &Inventory,
        who: usize,
        tradeslots: &[TradeSlot],
    ) {
        let mut slot_maker = SlotMaker {
            empty_slot: self.imgs.inv_slot,
            filled_slot: self.imgs.inv_slot,
            selected_slot: self.imgs.inv_slot_sel,
            background_color: Some(UI_MAIN),
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.75,
            },
            selected_content_scale: 1.067,
            amount_font: self.fonts.cyri.conrod_id,
            amount_margins: Vec2::new(-4.0, 0.0),
            amount_font_size: self.fonts.cyri.scale(12),
            amount_text_color: TEXT_COLOR,
            content_source: inventory,
            image_source: self.item_imgs,
            slot_manager: Some(self.slot_manager),
        };

        if state.ids.inv_slots.len() < 2 * MAX_TRADE_SLOTS {
            state.update(|s| {
                s.ids
                    .inv_slots
                    .resize(2 * MAX_TRADE_SLOTS, &mut ui.widget_id_generator());
            });
        }

        for i in 0..MAX_TRADE_SLOTS {
            let x = i % 4;
            let y = i / 4;

            let slot = tradeslots.get(i).cloned().unwrap_or(TradeSlot {
                index: i,
                quantity: 0,
                invslot: None,
            });
            // Slot
            let slot_widget = slot_maker
                .fabricate(slot, [40.0; 2])
                .top_left_with_margins_on(
                    state.ids.inv_alignment[who],
                    0.0 + y as f64 * (40.0),
                    0.0 + x as f64 * (40.0),
                );
            slot_widget.set(state.ids.inv_slots[i + who * MAX_TRADE_SLOTS], ui);
        }
    }

    fn phase2_itemwidget(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        inventory: &Inventory,
        who: usize,
        tradeslots: &[TradeSlot],
    ) {
        if state.ids.inv_textslots.len() < 2 * MAX_TRADE_SLOTS {
            state.update(|s| {
                s.ids
                    .inv_textslots
                    .resize(2 * MAX_TRADE_SLOTS, &mut ui.widget_id_generator());
            });
        }
        for i in 0..MAX_TRADE_SLOTS {
            let slot = tradeslots.get(i).cloned().unwrap_or(TradeSlot {
                index: i,
                quantity: 0,
                invslot: None,
            });
            let itemname = slot
                .invslot
                .and_then(|i| inventory.get(i))
                .map(|i| i.name())
                .unwrap_or("");
            let is_present = slot.quantity > 0 && slot.invslot.is_some();
            Text::new(&format!("{} x {}", slot.quantity, itemname))
                .top_left_with_margins_on(state.ids.inv_alignment[who], 10.0 + i as f64 * 30.0, 0.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .color(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    if is_present { 1.0 } else { 0.0 },
                ))
                .set(state.ids.inv_textslots[i + who * MAX_TRADE_SLOTS], ui);
        }
    }

    fn accept_decline_buttons(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
    ) -> <Self as Widget>::Event {
        let mut event = None;
        if Button::image(self.imgs.button)
            .w_h(31.0 * 5.0, 12.0 * 2.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .bottom_left_with_margins_on(state.ids.bg, 80.0, 60.0)
            .label(&self.localized_strings.get("hud.trade.accept"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.accept_button, ui)
            .was_clicked()
        {
            if trade.in_phase1() {
                event = Some(TradeActionMsg::Phase1Accept);
            } else if trade.in_phase2() {
                event = Some(TradeActionMsg::Phase2Accept);
            }
        }

        if Button::image(self.imgs.button)
            .w_h(31.0 * 5.0, 12.0 * 2.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .right_from(state.ids.accept_button, 20.0)
            .label(&self.localized_strings.get("hud.trade.decline"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.decline_button, ui)
            .was_clicked()
        {
            event = Some(TradeActionMsg::Decline);
        }
        event
    }

    fn close_button(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
    ) -> <Self as Widget>::Event {
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.bg, 0.0, 0.0)
            .set(state.ids.trade_close, ui)
            .was_clicked()
        {
            Some(TradeActionMsg::Decline)
        } else {
            None
        }
    }
}

impl<'a> Widget for Trade<'a> {
    type Event = Option<TradeActionMsg>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { mut state, ui, .. } = args;

        let mut event = None;
        let trade = match self.client.pending_trade() {
            Some((_, trade)) => trade,
            None => return Some(TradeActionMsg::Decline),
        };

        if state.ids.inv_alignment.len() < 2 {
            state.update(|s| {
                s.ids.inv_alignment.resize(2, &mut ui.widget_id_generator());
            });
        }
        if state.ids.offer_headers.len() < 2 {
            state.update(|s| {
                s.ids.offer_headers.resize(2, &mut ui.widget_id_generator());
            });
        }
        if state.ids.accept_indicators.len() < 2 {
            state.update(|s| {
                s.ids
                    .accept_indicators
                    .resize(2, &mut ui.widget_id_generator());
            });
        }

        // TODO: item tooltips in trade preview
        /*let trade_tooltip = Tooltip::new({
            // Edge images [t, b, r, l]
            // Corner images [tr, tl, br, bl]
            let edge = &self.rot_imgs.tt_side;
            let corner = &self.rot_imgs.tt_corner;
            ImageFrame::new(
                [edge.cw180, edge.none, edge.cw270, edge.cw90],
                [corner.none, corner.cw270, corner.cw90, corner.cw180],
                Color::Rgba(0.08, 0.07, 0.04, 1.0),
                5.0,
            )
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);*/

        self.background(&mut state, ui);
        self.title(&mut state, ui);
        self.phase_indicator(&mut state, ui, &trade);

        event = self.item_pane(&mut state, ui, &trade, 0).or(event);
        event = self.item_pane(&mut state, ui, &trade, 1).or(event);
        event = self
            .accept_decline_buttons(&mut state, ui, &trade)
            .or(event);
        event = self.close_button(&mut state, ui).or(event);

        event
    }
}
