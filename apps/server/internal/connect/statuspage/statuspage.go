package statuspage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"connectrpc.com/connect"
	"go.uber.org/zap"
	emptypb "google.golang.org/protobuf/types/known/emptypb"

	statuspagev1 "github.com/Obiente/Uppe/apps/server/gen/statuspage/v1"
	"github.com/Obiente/Uppe/apps/server/gen/statuspage/v1/statuspagev1connect"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	"github.com/Obiente/Uppe/apps/server/internal/models"
)

type StatusPageService struct {
	logger   *zap.Logger
	database db.Database
}

var _ statuspagev1connect.StatusPageServiceHandler = (*StatusPageService)(nil)

func NewStatusPageService(logger *zap.Logger, database db.Database) *StatusPageService {
	return &StatusPageService{
		logger:   logger,
		database: database,
	}
}

func (s *StatusPageService) CreateStatusPage(
	ctx context.Context,
	req *connect.Request[statuspagev1.CreateStatusPageRequest],
) (*connect.Response[statuspagev1.StatusPage], error) {
	if err := validatePage(req.Msg.Title, req.Msg.Slug, req.Msg.Description, req.Msg.LogoUrl, req.Msg.PrimaryColor, req.Msg.MonitorIds); err != nil {
		return nil, connect.NewError(connect.CodeInvalidArgument, err)
	}

	page := &models.StatusPage{
		Title:        req.Msg.Title,
		Slug:         req.Msg.Slug,
		Description:  req.Msg.Description,
		MonitorIDs:   req.Msg.MonitorIds,
		IsActive:     req.Msg.IsActive,
		PrimaryColor: req.Msg.PrimaryColor,
	}
	if req.Msg.LogoUrl != "" {
		page.LogoURL = &req.Msg.LogoUrl
	}

	if err := s.database.CreateStatusPage(ctx, page); err != nil {
		s.logger.Error("Failed to create status page", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to create status page"))
	}

	stored, err := s.database.GetStatusPage(ctx, page.ID, false)
	if err != nil {
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to reload status page"))
	}
	return connect.NewResponse(toProto(stored)), nil
}

func (s *StatusPageService) GetStatusPage(
	ctx context.Context,
	req *connect.Request[statuspagev1.GetStatusPageRequest],
) (*connect.Response[statuspagev1.StatusPage], error) {
	identifier := ""
	bySlug := false
	switch v := req.Msg.Identifier.(type) {
	case *statuspagev1.GetStatusPageRequest_Id:
		identifier = v.Id
	case *statuspagev1.GetStatusPageRequest_Slug:
		identifier = v.Slug
		bySlug = true
	}
	if identifier == "" {
		return nil, connect.NewError(connect.CodeInvalidArgument, fmt.Errorf("status page identifier is required"))
	}

	page, err := s.database.GetStatusPage(ctx, identifier, bySlug)
	if err != nil {
		s.logger.Error("Failed to load status page", zap.String("identifier", identifier), zap.Error(err))
		return nil, readFailure(err)
	}

	return connect.NewResponse(toProto(page)), nil
}

func (s *StatusPageService) ListStatusPages(
	ctx context.Context,
	req *connect.Request[statuspagev1.ListStatusPagesRequest],
) (*connect.Response[statuspagev1.ListStatusPagesResponse], error) {
	pages, total, err := s.database.ListStatusPages(ctx, int(req.Msg.Page), int(req.Msg.PageSize), req.Msg.ActiveOnly)
	if err != nil {
		s.logger.Error("Failed to list status pages", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to list status pages"))
	}

	response := &statuspagev1.ListStatusPagesResponse{
		StatusPages: make([]*statuspagev1.StatusPage, 0, len(pages)),
		Total:       int32(total),
	}
	for _, page := range pages {
		response.StatusPages = append(response.StatusPages, toProto(page))
	}
	return connect.NewResponse(response), nil
}

func (s *StatusPageService) UpdateStatusPage(
	ctx context.Context,
	req *connect.Request[statuspagev1.UpdateStatusPageRequest],
) (*connect.Response[statuspagev1.StatusPage], error) {
	update := &models.StatusPageUpdate{
		Title:        req.Msg.Title,
		Slug:         req.Msg.Slug,
		Description:  req.Msg.Description,
		IsActive:     req.Msg.IsActive,
		LogoURL:      req.Msg.LogoUrl,
		PrimaryColor: req.Msg.PrimaryColor,
	}
	if req.Msg.ReplaceMonitorIds || len(req.Msg.MonitorIds) > 0 {
		update.MonitorIDs = req.Msg.MonitorIds
		update.ReplaceMonitorIDs = true
	}
	current, err := s.database.GetStatusPage(ctx, req.Msg.Id, false)
	if err != nil {
		return nil, readFailure(err)
	}
	if update.Title != nil {
		current.Title = *update.Title
	}
	if update.Slug != nil {
		current.Slug = *update.Slug
	}
	if update.Description != nil {
		current.Description = *update.Description
	}
	if update.LogoURL != nil {
		current.LogoURL = update.LogoURL
	}
	if update.PrimaryColor != nil {
		current.PrimaryColor = *update.PrimaryColor
	}
	if update.ReplaceMonitorIDs {
		current.MonitorIDs = update.MonitorIDs
	}
	logo := ""
	if current.LogoURL != nil {
		logo = *current.LogoURL
	}
	if err := validatePage(current.Title, current.Slug, current.Description, logo, current.PrimaryColor, current.MonitorIDs); err != nil {
		return nil, connect.NewError(connect.CodeInvalidArgument, err)
	}

	if err := s.database.UpdateStatusPage(ctx, req.Msg.Id, update); err != nil {
		s.logger.Error("Failed to update status page", zap.String("id", req.Msg.Id), zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to update status page"))
	}

	page, err := s.database.GetStatusPage(ctx, req.Msg.Id, false)
	if err != nil {
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to reload status page"))
	}
	return connect.NewResponse(toProto(page)), nil
}

func (s *StatusPageService) DeleteStatusPage(
	ctx context.Context,
	req *connect.Request[statuspagev1.DeleteStatusPageRequest],
) (*connect.Response[emptypb.Empty], error) {
	if err := s.database.DeleteStatusPage(ctx, req.Msg.Id); err != nil {
		s.logger.Error("Failed to delete status page", zap.String("id", req.Msg.Id), zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to delete status page"))
	}
	return connect.NewResponse(&emptypb.Empty{}), nil
}

func (s *StatusPageService) RecordVisit(
	ctx context.Context,
	req *connect.Request[statuspagev1.RecordVisitRequest],
) (*connect.Response[emptypb.Empty], error) {
	if err := s.database.RecordStatusPageVisit(ctx, req.Msg.StatusPageId); err != nil {
		s.logger.Error("Failed to record status page visit", zap.String("id", req.Msg.StatusPageId), zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to record status page visit"))
	}
	return connect.NewResponse(&emptypb.Empty{}), nil
}

func toProto(page *models.StatusPage) *statuspagev1.StatusPage {
	proto := &statuspagev1.StatusPage{
		Id:           page.ID,
		Title:        page.Title,
		Slug:         page.Slug,
		MonitorIds:   page.MonitorIDs,
		IsActive:     page.IsActive,
		PrimaryColor: page.PrimaryColor,
		Description:  page.Description,
		Uptime:       page.Uptime,
		Visits:       page.Visits,
		CreatedAt:    page.CreatedAt.Unix(),
		UpdatedAt:    page.UpdatedAt.Unix(),
	}
	if page.CustomDomain != nil {
		proto.CustomDomain = *page.CustomDomain
	}
	if page.LogoURL != nil {
		proto.LogoUrl = *page.LogoURL
	}
	if page.LastIncident != nil {
		timestamp := page.LastIncident.Unix()
		proto.LastIncident = &timestamp
	}
	return proto
}

func readFailure(err error) error {
	if errors.Is(err, sql.ErrNoRows) {
		return connect.NewError(connect.CodeNotFound, fmt.Errorf("status page not found"))
	}
	return connect.NewError(connect.CodeUnavailable, fmt.Errorf("status page storage is unavailable"))
}
