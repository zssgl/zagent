set @companieId='54609e06-cf0b-4501-9597-1a01b4bbf62a';
set @clinicId='';
set @startDay='2010-01-01';
set @endDay=CURDATE();

DROP TABLE IF EXISTS bi.report_consumption_detail_temp;

-- Step 2: Create the table structure
CREATE TABLE bi.report_consumption_detail_temp (
    `顾客姓名` VARCHAR(255),
    `病历编号` VARCHAR(255),
    `性别` VARCHAR(10),
    `消耗时间` DATETIME,
    `类型` VARCHAR(255),
    `业绩人` VARCHAR(255),
    `项目分类` VARCHAR(255),
		`项目ID` VARCHAR(255),
    `项目商品` VARCHAR(255),
    `消耗数量` INT,
    `消耗金额` DECIMAL(15, 2),
    `消耗诊所` VARCHAR(255),
    `创建人` VARCHAR(255),
    `开单诊所` VARCHAR(255),
    `实质消费` DECIMAL(15, 2),
    `消耗金额（非欠费均分）` DECIMAL(15, 2),
    `促销活动` VARCHAR(255),
    `组合项目` VARCHAR(255),
    `提点类型` VARCHAR(255),
    `最早开单时间` DATETIME,
    `订单来源` INT,
    `是否是内购订单` INT
);

-- Step 3: Insert data into the table
INSERT INTO bi.report_consumption_detail_temp (
    `顾客姓名`, `病历编号`, `性别`, `消耗时间`, `类型`, `业绩人`, 
    `项目分类`,`项目ID`, `项目商品`, `消耗数量`, `消耗金额`, `消耗诊所`, 
    `创建人`, `开单诊所`, `实质消费`, `消耗金额（非欠费均分）`, 
    `促销活动`, `组合项目`, `提点类型`,`最早开单时间`,`订单来源`,`是否是内购订单`
)
SELECT
  -- coursedetail.`bill_id`,
  -- coursedetail.`bill_no` AS `订单编号`,
  customer.`customer_name` AS `顾客姓名`,
  customer.`customer_cid` AS `病历编号`,
  customer.`customer_sex` AS `性别`,
  coursedetail.`item_course_time` AS `消耗时间`,
  coursedetail.`item_type_name` AS `类型`,
  coursedetail.`performers` AS `业绩人`,
  coursedetail.`item_category_name` AS `项目分类`,
	coursedetail.`item_id` AS `项目ID`,
  coursedetail.`item_name` AS `项目商品`,
  coursedetail.`item_course_count` AS `消耗数量`,
  coursedetail.`item_course_amount` / 100 AS `消耗金额`,
  coursedetail.`course_clinic_name` AS `消耗诊所`,
  coursedetail.`creator_name` AS `创建人`,
  coursedetail.`bill_clinic_name` AS `开单诊所`,
  coursedetail.`item_course_real_amount` / 100 AS `实质消费`,
  coursedetail.`item_course_amount` / 100 AS `消耗金额（非欠费均分）`,
  CASE 
    WHEN coursedetail.`item_source_type` = 2 THEN coursedetail.`item_source_name` 
    ELSE '' 
  END AS `促销活动`,
  CASE 
    WHEN coursedetail.`item_source_type` = 1 AND coursedetail.`item_source_service_is_group` = 1 
    THEN coursedetail.`item_source_name` 
    ELSE '' 
  END AS `组合项目`,
  coursedetail.`bill_classify_name` AS `提点类型`,
  customer_exitr.`first_real_pay_time` as `最早开单时间`,
  coursedetail.`bill_source` AS `订单来源`,
  coursedetail.`in_app_purchase` AS `是否是内购订单`
from
  (
    /*
    开单商品
    */
    select
      billproduct.`bill_item_id` as `course_detail_id`,
      bill.`bill_clinic_id` as `course_clinic_id`,
      bill.`bill_clinic_name` as `course_clinic_name`,
      bill.`bill_pay_time` as `item_course_time`,
      
      billproduct.`bill_id`,
      billproduct.`bill_detail_id`,
      billproduct.`item_source_id`,
      billproduct.`item_source_name`,
      billproduct.`item_source_type`,
      billproduct.`item_source_type_name`,
      billproduct.`item_source_service_is_group`,
      
      bill.`bill_no`,
      bill.`bill_clinic_id`,
      bill.`bill_clinic_name`,
      bill.`customer_id`,
      
      billproduct.`item_type`,
      billproduct.`item_type_name`,
      billproduct.`item_category_id`,
      billproduct.`item_category_name`,
      billproduct.`item_category_parent_id`,
      billproduct.`item_category_parent_name`,
      billproduct.`item_category_name` as `item_category_track`,
      billproduct.`item_id`,
      billproduct.`item_name`,
      0 as `item_workpoints`,
      billproduct.`item_count` as `course_sum_count`,
      billproduct.`item_pay_amount` as `course_sum_amount`,
      billproduct.`bill_classify_id`,
      billproduct.`bill_classify_name`,
      billproduct.`course_classify_id`,
      billproduct.`course_classify_name`,
      billproduct.`item_count` as `item_course_count`,
      billproduct.`item_pay_amount` as `item_course_amount`,
      billproduct.`item_pay_real_amount` as `item_course_real_amount`,
      bill.`performers`,
      bill.`performers_details`,
      bill.`creator_id`,
      bill.`creator_name`,

      billproduct.`is_cool_sculpting`,
      billproduct.`bill_source`,
      billproduct.`in_app_purchase`
    from
      (
        /*
        商品
        */
        select
          t.`bill_id`,
                
          billdetail.`ID` as `bill_detail_id`,
          billdetail.`ItemId` as `item_source_id`,
          billdetail.`ItemName` as `item_source_name`,
          billdetail.`CareType` as `item_source_type`,
          case billdetail.`CareType` when 0 then '服务项目' when 1 then '商品' else '促销活动' end as `item_source_type_name`,
          case billdetail.`CareType` when 0 then (select ifnull(cs.`IsGroupItem`, 0) as `item_source_service_is_group` from careservices cs where cs.`ID`=billdetail.`ItemId`) else 0 end as `item_source_service_is_group`,
          billdetail.`is_network_order`,
          case billdetail.`CareType` when 2 then (select promotion.`is_promotion` from promotions promotion where promotion.`ID`=billdetail.`ItemId`) else 0 end as `is_promotion`,
          
          
          billproduct.`ID` as `bill_item_id`,
          1 as `item_type`,
          '商品' as `item_type_name`,
          good.`ID` as `item_id`,
          good.`Name` as `item_name`,
          goodscategory.`ID` as `item_category_id`,
          goodscategory.`Name` as `item_category_name`,
          goodscategoryparent.`ID` as `item_category_parent_id`,
          goodscategoryparent.`Name` as `item_category_parent_name`,
          good.`Specification` as `item_specification`,
          good.`Unit` as `item_unit`,
          billproduct.`ItemCnt` as `item_count`,
          billproduct.`PayAmount` as `item_pay_amount`,
          billproduct.`pay_real_amount` as `item_pay_real_amount`,
          goodscategory.`ID` as `bill_classify_id`,
          goodscategory.`Name` as `bill_classify_name`,
          goodscategory.`ID` as `course_classify_id`,
          goodscategory.`Name` as `course_classify_name`,
          0 as `is_cool_sculpting`,
          billproduct.`refund_real_amount` as `item_refund_real_amount`,
          bill.`source` as `bill_source`,
          bill.`in_app_purchase`
        from
          (
            select
              t.`bill_id`
            from
              (
                select
                  bill.`bill_id`,
                  min(paybook.`PayTime`) as `first_pay_time`
                from
                  (
                    select
                      paybook.`BillId` as `bill_id`
                    from
                      paybooks paybook
                    where
                      paybook.`CompId`=@companieId
                      and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
                      and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                      and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                    group by
                      paybook.`BillId`
                  ) bill
                  left join paybooks paybook on paybook.`BillId`=bill.`bill_id`
                group by
                  bill.`bill_id`
              ) t
              left join bills bill on bill.`ID`=t.`bill_id`
            where
              bill.`BillStatus`!=4
              and DATE_FORMAT(t.`first_pay_time`, '%Y-%m-%d')>=@startDay
              and DATE_FORMAT(t.`first_pay_time`, '%Y-%m-%d')<=@endDay
          ) t
          left join bills bill on bill.`ID`=t.`bill_id`
          left join billdetails billdetail on billdetail.`Bill_ID`=bill.`ID`
          left join billproducts billproduct on billproduct.`BillDetailiId`=billdetail.`ID`
          left join goods good on good.`ID`=billproduct.`GoodsId`
          left join goodscategorys goodscategory on goodscategory.`ID`=good.`GoodsCategory_ID`
          left join goodscategorys goodscategoryparent on goodscategoryparent.`ID`=goodscategory.`ParentCategory_ID`
        where
          billproduct.`BillDetailiId` is not null
      ) billproduct
      left join (
        /*
        订单统计
        */
        select
          t.`bill_id`,
          bill.`BillNo` as `bill_no`,
          bill.`ClinicId` as `bill_clinic_id`,
          billclinic.`Name` as `bill_clinic_name`,
          bill.`Customer_ID` as `customer_id`,
          case visit.`Type` when 1 then '初诊' when 2 then '复诊' when 3 then '疗程内' else '再消费' end as `visit_consumption_type`,
          bill.`Visit_ID` as `bill_visit_id`,
          visit.`ConsultantId` as `visit_consultant_id`,
          visit.`ConsultantName` as `visit_consultant_name`,
          bill.`DiscountedAmount` as `bill_discounted_amount`,
          group_concat(employee.`EmpName` separator ',') as `performers`,
          group_concat(CONCAT(employee.`ID`, '|', employee.`EmpName`) separator ';') as `performers_details`,
          bill.`CreateEmpId` as `creator_id`,
          creator.`EmpName` as `creator_name`,
          bill.`CreateTime` as `bill_create_time`,
          ifnull(bill.`Memo`,'') as `bill_memo`,
          
          t.`pay_book_id`,
          t.`pay_clinic_id`,
          t.`pay_clinic_name`,
          t.`bill_pay_amount`,
          bill.`pay_real_amount` as `bill_pay_real_amount`,
          bill.`RefundAmount` as `refund_amount`,
          bill.`refund_real_amount` as `refund_real_amount`,
          bill.`source` as `bill_source`,
          bill.`in_app_purchase`,
          bill.`external_id`,
          bill.`integral`,
          bill.`deduction_amount`,
          t.`bill_pay_time`,
          t.`payment_methods`,
          t.`payment_check_off_code`
        from
          (
            select
              t.`bill_id`,
              
              paybook.`ID` as `pay_book_id`,
              paybook.`OrganizationId` as `pay_clinic_id`,
              payclinic.`Name` as `pay_clinic_name`,
              t.`bill_pay_amount`,
              t.`payment_methods`,
              t.`payment_check_off_code`,
              t.`first_pay_time` as `bill_pay_time`
            from
              (
                select
                  bill.`bill_id`,
                  min(paybook.`PayTime`) as `first_pay_time`,
                  sum(paybookchannel.`PayAmount`) as `bill_pay_amount`,
                  group_concat(CONCAT(paybookchannel.`ChannelName`, ':', TRUNCATE((paybookchannel.`PayAmount` / 100), 2)) separator ';') as `payment_methods`,
                  group_concat(pay_book_channel_check_off_code.`check_off_code` separator ';') as `payment_check_off_code`
                from
                  (
                    select
                      paybook.`BillId` as `bill_id`
                    from
                      paybooks paybook
                    where
                      paybook.`CompId`=@companieId
                      and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
                      and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
                      and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
                    group by
                      paybook.`BillId`
                  ) bill
                  left join paybooks paybook on paybook.`BillId`=bill.`bill_id`
                  left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`ID`
                  left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
                  left join pay_book_channel_check_off_codes pay_book_channel_check_off_code on pay_book_channel_check_off_code.`pay_book_channel_id`=paybookchannel.`ID`
                group by
                  bill.`bill_id`
              ) t
              left join bills bill on bill.`ID`=t.`bill_id`
              left join paybooks paybook on paybook.`BillId`=t.`bill_id` and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d %H:%i:%s')=DATE_FORMAT(t.`first_pay_time`, '%Y-%m-%d %H:%i:%s')
              left join organizations payclinic on payclinic.`ID`=paybook.`OrganizationId`
            where
              bill.`BillStatus`!=4
              and DATE_FORMAT(t.`first_pay_time`, '%Y-%m-%d')>=@startDay
              and DATE_FORMAT(t.`first_pay_time`, '%Y-%m-%d')<=@endDay
          ) t
          left join bills bill on bill.`ID`=t.`bill_id`
          left join organizations billclinic on billclinic.`ID`=bill.`ClinicId`
          left join billemployees billemployee on billemployee.`BillId`=bill.`ID`
          left join employees employee on employee.`ID`=billemployee.`EmpId`
          left join visits visit on visit.`ID`=bill.`Visit_ID`
          left join employees creator on creator.`ID`=bill.`CreateEmpId`
        group by
          t.`bill_id`
      ) bill on bill.`bill_id`=billproduct.`bill_id`


    union all


    /*
    消耗项目
    */
    select
      coursedetail.`course_detail_id`,
      coursedetail.`course_clinic_id`,
      coursedetail.`course_clinic_name`,
      coursedetail.`item_course_time`,
      
      coursedetail.`bill_id`,
      coursedetail.`bill_detail_id`,
      coursedetail.`item_source_id`,
      coursedetail.`item_source_name`,
      coursedetail.`item_source_type`,
      coursedetail.`item_source_type_name`,
      coursedetail.`item_source_service_is_group`,
      
      coursedetail.`bill_no`,
      coursedetail.`bill_clinic_id`,
      coursedetail.`bill_clinic_name`,
      coursedetail.`customer_id`,
      
      coursedetail.`item_type`,
      coursedetail.`item_type_name`,
      coursedetail.`item_category_id`,
      coursedetail.`item_category_name`,
      coursedetail.`item_category_parent_id`,
      coursedetail.`item_category_parent_name`,
      coursedetail.`item_category_track`,
      coursedetail.`item_id`,
      coursedetail.`item_name`,
      coursedetail.`item_workpoints`,
      coursedetail.`course_sum_count`,
      coursedetail.`course_sum_amount`,
      coursedetail.`bill_classify_id`,
      coursedetail.`bill_classify_name`,
      coursedetail.`course_classify_id`,
      coursedetail.`course_classify_name`,
      coursedetail.`item_course_count`,
      coursedetail.`item_course_amount`,
      coursedetail.`item_course_real_amount`,
      group_concat(courseemployee.`EmpName` separator ',') as `performers`,
      group_concat(CONCAT(courseemployee.`ID`, '|', courseemployee.`EmpName`) separator ';') as `performers_details`,
      coursedetail.`creator_id`,
      coursedetail.`creator_name`,

      coursedetail.`is_cool_sculpting`,
      coursedetail.`bill_source`,
      coursedetail.`in_app_purchase`
    from
      (
        select
          coursedetail.*,
          sum(courseline.`pay_real_amount`) as `item_course_real_amount`
        from
          (
            select
              coursedetail.`ID` as `course_detail_id`,
              coursedetail.`ClinicId` as `course_clinic_id`,
              courseclinic.`Name` as `course_clinic_name`,
              coursedetail.`CreateTime` as `item_course_time`,
              
              billdetail.`Bill_ID` as `bill_id`,
              course.`BillDetailiId` as `bill_detail_id`,
              billdetail.`ItemId` as `item_source_id`,
              billdetail.`ItemName` as `item_source_name`,
              billdetail.`CareType` as `item_source_type`,
              case billdetail.`CareType` when 0 then '服务项目' when 1 then '商品' else '促销活动' end as `item_source_type_name`,
              case billdetail.`CareType` when 0 then (select ifnull(cs.`IsGroupItem`, 0) as `item_source_service_is_group` from careservices cs where cs.`ID`=billdetail.`ItemId`) else 0 end as `item_source_service_is_group`,
              
              bill.`BillNo` as `bill_no`,
              billdetail.`ClinicId` as `bill_clinic_id`,
              billclinic.`Name` as `bill_clinic_name`,
              course.`Customer_ID` as `customer_id`,
              
              0 as `item_type`,
              '项目' as `item_type_name`,
              itemcategory.`ID` as `item_category_id`,
              itemcategory.`Name` as `item_category_name`,
              itemCategoryParent.`ID` as `item_category_parent_id`,
              itemCategoryParent.`Name` as `item_category_parent_name`,
              CONCAT(itemCategoryParent.`Name`, ' - ', itemcategory.`Name`) as `item_category_track`,
              careservice.`ID` as `item_id`,
              careservice.`Name` as `item_name`,
              careservice.`workpoints` as `item_workpoints`,
              course.`SumCourseQty` as `course_sum_count`,
              course.`SumAmount` as `course_sum_amount`,
              careservice.`bill_classify_id`,
              csbc.`Name` as `bill_classify_name`,
              careservice.`course_classify_id`,
              cscc.`Name` as `course_classify_name`,
              coursedetail.`CourseCount` as `item_course_count`,
              coursedetail.`CourseAmount` as `item_course_amount`,
              employee.`ID` as `creator_id`,
              employee.`EmpName` as `creator_name`,
              
              case when careservice.`bill_classify_id`='86ca40ac-d533-405e-017a-8f5313cd207f' or careservice.`bill_classify_id`='6ac32a4b-6e44-900f-5477-69a943a30706' then 1 else 0 end as `is_cool_sculpting`,
              bill.`source` as `bill_source`,
              bill.`in_app_purchase`
            from
              coursedetails coursedetail
              left join courses course on course.`ID`=coursedetail.`Course_ID`
              left join billdetails billdetail on billdetail.`ID`=course.`BillDetailiId`
              left join bills bill on bill.`ID`=billdetail.`Bill_ID`
              left join careservices careservice on careservice.`ID`=course.`ServiceId`
              left join itemcategorys itemcategory on itemcategory.`ID`=careservice.`Category_ID`
              left join itemcategorys itemCategoryParent on itemCategoryParent.`ID`=itemcategory.`ParentId`
              left join organizations courseclinic on courseclinic.`ID`=coursedetail.`ClinicId`
              left join organizations billclinic on billclinic.`ID`=course.`ClinicId`
              left join employees employee on employee.`ID`=coursedetail.`CreateUserID`
              left join care_services_bill_classifys csbc on csbc.`ID`=careservice.`bill_classify_id`
              left join care_services_course_classifys cscc on cscc.`ID`=careservice.`course_classify_id`
            where
              coursedetail.`CompId`=@companieId
              and if(@clinicId='0', 1=1, if(@clinicId='',1=1,coursedetail.`ClinicId`=@clinicId))
              and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')>=@startDay
              and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')<=@endDay
          ) coursedetail
          left join coursedetaillines coursedetailline on coursedetailline.`CourseDetailId`=coursedetail.`course_detail_id`
          left join courselines courseline on courseline.`ID`=coursedetailline.`CourseLineId`
        group by
          coursedetail.`course_detail_id`
      ) coursedetail
      left join employeecoursedetails employeecoursedetail on employeecoursedetail.`CourseDetailId`=coursedetail.`course_detail_id`
      left join employees courseemployee on courseemployee.`ID`=employeecoursedetail.`EmployeeId`
    group by
      coursedetail.`course_detail_id`
  ) coursedetail
  left join (
    select
      t.`customer_id`,
      customer.`Name` as `customer_name`,
      case customer.`Sex` when 1 then '男' when 2 then '女' else '' end as `customer_sex`,
      customer.`CID` as `customer_cid`
    from
      (
        select
          bill.`Customer_ID` as `customer_id`
        from
          paybooks paybook
          left join bills bill on bill.`ID`=paybook.`BillId`
        where
          paybook.`CompId`=@companieId
          and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
          and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
          and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
        group by
          bill.`Customer_ID`

        union

        select
          course.`Customer_ID` as `customer_id`
        from
          coursedetails coursedetail
          left join courses course on course.`ID`=coursedetail.`Course_ID`
        where
          coursedetail.`CompId`=@companieId
          and if(@clinicId='0', 1=1, if(@clinicId='',1=1,coursedetail.`ClinicId`=@clinicId))
          and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')>=@startDay
          and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')<=@endDay
        group by
          course.`Customer_ID`
      ) t
      left join customers customer on customer.`ID`=t.`customer_id`
      left join organizations clinic on clinic.`ID`=customer.`OrganizationID`
  ) customer on customer.`customer_id`=coursedetail.`customer_id`
  left join (
    select
      t.`customer_id`,
      min(paybook.`PayTime`) as `first_real_pay_time`
    from
      (
        select
          bill.`Customer_ID` as `customer_id`
        from
          paybooks paybook
          left join bills bill on bill.`ID`=paybook.`BillId`
        where
          paybook.`CompId`=@companieId
          and if(@clinicId='0', 1=1, if(@clinicId='',1=1,paybook.`OrganizationId`=@clinicId))
          and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')>=@startDay
          and DATE_FORMAT(paybook.`PayTime`, '%Y-%m-%d')<=@endDay
        group by
          bill.`Customer_ID`

        union

        select
          course.`Customer_ID` as `customer_id`
        from
          coursedetails coursedetail
          left join courses course on course.`ID`=coursedetail.`Course_ID`
        where
          coursedetail.`CompId`=@companieId
          and if(@clinicId='0', 1=1, if(@clinicId='',1=1,coursedetail.`ClinicId`=@clinicId))
          and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')>=@startDay
          and DATE_FORMAT(coursedetail.`CreateTime`, '%Y-%m-%d')<=@endDay
        group by
          course.`Customer_ID`
      ) t
      left join bills bill on bill.`Customer_ID`=t.`customer_id`
      left join paybooks paybook on paybook.`BillId`=bill.`ID`
      left join paybookchannels paybookchannel on paybookchannel.`PayBookId`=paybook.`ID`
      left join paymentmethods paymentmethod on paymentmethod.`ID`=paybookchannel.`ChannelId`
    where
      paymentmethod.`IsRealConsumption`=1 or paymentmethod.`ID`='6756de43-f712-c80b-9687-08d59e04cec0'
    group by
      t.`customer_id`
  ) customer_exitr on customer_exitr.`customer_id`=coursedetail.`customer_id`

order by
  coursedetail.`item_course_time` desc
